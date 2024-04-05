use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use kdam::{tqdm, Bar, BarExt};
use pbkdf2::pbkdf2_hmac;
use regex::Regex;
use reqwest::{
    self,
    header::{self, HeaderMap},
    Client,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use std::{collections::LinkedList, fs};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use tokio::sync::Mutex;
use tokio::sync::{OwnedSemaphorePermit, RwLock, Semaphore};

mod convert;
use convert::convert_video_from_mpeg_to_mp4;

use crate::timer::{TimeRange, TimedDownload as _};

#[derive(Serialize, Deserialize)]
pub struct VideoQuality {
    name: String,
    size: String,
}

#[derive(Serialize, Deserialize)]
pub struct VideoData {
    title: String,
    playlist: String,
    msgn: String,
    download: Vec<VideoQuality>,
}

struct VideoInner {
    video_id: String,
    video_host: String,
    desired_quality: Option<String>,
    quality_index: usize,
    time_range: Option<TimeRange>,
    client: Client,
}

#[derive(Clone)]
pub struct Video {
    inner: Arc<RwLock<VideoInner>>,
}

impl Video {
    pub async fn print_extracted(&self) {
        let inner = self.inner.read().await;
        println!("[Video host] {}", &inner.video_host);
        println!("[Video id] {}", &inner.video_id);
    }

    pub async fn set_time_range(&mut self, time_range: TimeRange) {
        self.inner.write().await.time_range = Some(time_range);
    }

    pub fn new(video_id: String, video_host: String, desired_quality: Option<String>) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::REFERER,
            format!("https://{}/{}/iframe", &video_host, &video_id)
                .parse()
                .unwrap(),
        );

        let client = reqwest::ClientBuilder::new()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:122.0) Gecko/20100101 Firefox/122.0",
            )
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            inner: Arc::new(RwLock::new(VideoInner {
                video_id,
                video_host,
                quality_index: 0,
                desired_quality,
                time_range: None,
                client,
            })),
        }
    }

    fn decrypt_m3u8(msgn: &str, m3u8_text: &str) -> Result<String, Box<dyn std::error::Error>> {
        let encrypted_string = String::from_utf8(STANDARD.decode(m3u8_text)?)?;
        let mut parts = encrypted_string.split('-');
        let first_part = parts.next().ok_or("first m3u8 part missing")?;
        let secret = format!("{}{}", msgn, first_part);
        let secret = secret.as_bytes();
        let salt = hex::decode(parts.next().ok_or("salt exctraction failed")?)?;
        let mut key = [0_u8; 32];
        pbkdf2_hmac::<Sha256>(secret, &salt, 1000, &mut key);
        let key = Key::<Aes256Gcm>::from_slice(&key);
        let nonce = hex::decode(parts.next().ok_or("nonce extraction failed")?)?;
        let cipher = Aes256Gcm::new(&key);
        let data = hex::decode(parts.next().ok_or("data extraction failed")?)?;
        let decrypted = cipher.decrypt(nonce.as_slice().into(), data.as_slice());
        match decrypted {
            Ok(x) => {
                let result = String::from_utf8(x)?;
                return Ok(result);
            }
            Err(_) => {
                return Err("m3u8 decryption error".into());
            }
        }
    }

    pub async fn download(&self, is_in_batch: bool) -> Result<(), Box<dyn std::error::Error>> {
        let mut self_data = self.inner.write().await;

        let download_timer = self_data.time_range.clone();
        if !self_data.time_range.should_coutinue() {
            return Err("Timer went out of range".into());
        }

        println!("[Progress] fetching embed files");

        let embed_url = format!(
            "https://{}/{}/embed",
            &self_data.video_host, &self_data.video_id
        );

        let embed_res = self_data.client.get(&embed_url).send().await?;
        if embed_res.status() != 200 {
            println!("[Error] cannot get embed.js file");
            return Err("".into());
        }

        let embed_body = embed_res.text().await?;

        let regex = Regex::new(r"'.*?'").unwrap();
        let mut data_wraps = regex.find_iter(&embed_body);

        let embed_video_data: VideoData;

        match data_wraps.nth(24) {
            Some(matched) => {
                let base_64_json = matched.as_str();
                println!("[Embed data] {}", base_64_json);
                let base_64_json_str = &base_64_json[1..base_64_json.len() - 1];

                let decoded_json_string = STANDARD.decode(base_64_json_str)?;
                embed_video_data = serde_json::from_slice(&decoded_json_string)?;
            }
            None => {
                return Err("Cannot extract embed data".into());
            }
        }

        let mut safe_title = embed_video_data.title;
        for char in r#"\/:*?"<>|"#.chars() {
            safe_title = safe_title.replace(char, "-");
        }

        match fs::metadata(format!("{}.mp4", &safe_title)) {
            Ok(_) => {
                return Err("Video already downloaded".into());
            }
            Err(_) => (),
        }

        println!("[Progress] Fetching playlists");

        let playlist_url = format!(
            "https://{}/{}.m3u8",
            &self_data.video_host, &embed_video_data.playlist
        );

        let playlist_res = self_data.client.get(playlist_url).send().await?;
        if playlist_res.status() != 200 {
            println!("[Error] Cannot get playlist file");
            return Err("".into());
        }

        let encrypted_playlist_text = playlist_res.text().await?;

        let playlist_text = Self::decrypt_m3u8(&embed_video_data.msgn, &encrypted_playlist_text)?;
        if !is_in_batch && self_data.desired_quality.is_none() {
            println!("[Prompt] Select desired quality: ");
            for (index, video_quality) in embed_video_data.download.iter().enumerate() {
                println!("[Choice] {} -> {}", video_quality.name, index);
            }
        }

        let mut index_string = String::from("0");
        let mut valid_selection = false;
        let mut selected_playlist_link = "";
        let mut q_index = 0;
        if let Some(desired_quality) = &self_data.desired_quality {
            let desired_quality = desired_quality.to_owned() + "p";
            let found_index = embed_video_data
                .download
                .iter()
                .position(|x| x.name == desired_quality)
                .ok_or(format!(
                    "Specified quality {} is unavalable in video",
                    &desired_quality
                ))?;
            let target_line = (found_index + 1) * 2;
            match playlist_text.split('\n').nth(target_line) {
                Some(link) => {
                    valid_selection = true;
                    q_index = found_index;
                    selected_playlist_link = link;
                }
                None => {
                    // it's impossible because index is already found in embed data
                    return Err("This should never be shown to user (impossible error)".into());
                }
            }
        }
        while !valid_selection {
            if !is_in_batch {
                index_string.clear();
                std::io::stdin().read_line(&mut index_string)?;
                index_string = index_string.trim().to_string();
            }
            match index_string.parse::<usize>() {
                Ok(index) => {
                    let target_line = (index + 1) * 2;
                    match playlist_text.split('\n').nth(target_line) {
                        Some(link) => {
                            valid_selection = true;
                            q_index = index;
                            selected_playlist_link = link;
                        }
                        None => {
                            println!("[Error] Index out of range try again:");
                        }
                    }
                }
                Err(_) => {
                    println!("[Error] Cannot parse input to usize try again:");
                }
            }
            if is_in_batch && !valid_selection {
                return Err("Unable to get best quality".into());
            }
        }

        self_data.quality_index = q_index;

        let playlist_m3u8_res = self_data.client.get(selected_playlist_link).send().await?;
        if playlist_m3u8_res.status() != 200 {
            println!("[Error] Cannot get playlist parts");
            return Err("".into());
        }

        let encrypted_playlist_text = playlist_m3u8_res.text().await?;
        let playlist_text = Self::decrypt_m3u8(&embed_video_data.msgn, &encrypted_playlist_text)?;
        let lines = playlist_text.split('\n');

        let mut part_links = LinkedList::new();
        let mut cipher_iv = Vec::new();
        let mut cipher_key = Vec::new();

        for line in lines {
            if line.starts_with("#EXT-X-KEY") {
                match line.split("IV=").last() {
                    Some(iv_hex) => {
                        cipher_iv = hex::decode(&iv_hex[2..])?;
                    }
                    None => {
                        return Err("Cannot find stream iv".into());
                    }
                }

                match line.split('"').nth(1) {
                    Some(link) => {
                        let res = self_data.client.get(link).send().await?;
                        println!("[Progress] Key uri reponse code: '{}'", res.status());
                        if res.status() != 200 {
                            return Err("Key uri returned none 200 status".into());
                        }
                        cipher_key = res.bytes().await?.to_vec();
                    }
                    None => {
                        return Err("Cannot find stream key uri".into());
                    }
                }
            }
            if line.starts_with("https://") {
                part_links.push_back(line.to_string());
            }
        }

        let download_semaphore = Arc::new(Semaphore::new(10));
        let mut download_handles = Vec::new();

        let _ = fs::create_dir(&self_data.video_id);
        let arc_cipher_key = Arc::new(cipher_key);
        let arc_cipher_iv = Arc::new(cipher_iv);

        let total_size = embed_video_data.download[self_data.quality_index]
            .size
            .parse::<usize>()?;

        let pb = tqdm!(
            total = total_size,
            unit_scale = true,
            unit_divisor = 1024,
            unit = "B"
        );

        let pb = Arc::new(Mutex::new(pb));
        let directory_path = PathBuf::from(&self_data.video_id);
        drop(self_data);

        let mut index_counter = 0;
        while let Some(link) = part_links.pop_front() {
            if !download_timer.should_coutinue() {
                break;
            }
            let index = index_counter;
            index_counter += 1;
            let semaphore = download_semaphore.clone();
            let permit = semaphore.acquire_owned().await?;
            let iv = arc_cipher_iv.clone();
            let key = arc_cipher_key.clone();
            let pb = pb.clone();
            let fut = self.clone().download_part(index, link, permit, iv, key, pb);
            let handle = tokio::spawn(fut);
            download_handles.push(handle);
        }

        for handle in download_handles {
            handle.await?;
        }

        let self_data = self.inner.read().await;

        if !download_timer.should_coutinue() {
            return Err("Timer went out of range in middle of download".into());
        }

        println!("[Progress] Created mpeg video");

        let mut outfile = fs::File::create(directory_path.join("placeholder.mpeg"))?;
        for index in 0..index_counter {
            let name = Self::part_name(index, self_data.quality_index);
            let file_content = fs::read(directory_path.join(name))?;
            outfile.write_all(&file_content)?;
        }

        let input_file = directory_path.join("placeholder.mpeg\0");
        let input_file = input_file
            .to_str()
            .ok_or("Cannot convert PathBuf to &str")?;
        let output_file = format!("{}.mp4\0", &safe_title);

        unsafe {
            convert_video_from_mpeg_to_mp4(
                input_file.as_ptr() as *const libc::c_char,
                output_file.as_ptr() as *const libc::c_char,
            );
        }

        println!("[Progress] Mp4 video created");

        let _ = fs::remove_dir_all(directory_path);

        println!("[Progress] Directory deleted");

        Ok(())
    }

    fn part_name(index: usize, quality_index: usize) -> String {
        format!("Vpart-{:010}-{:02}.ts", index, quality_index)
    }

    async fn download_part(
        self,
        index: usize,
        link: String,
        _permit: OwnedSemaphorePermit,
        iv: Arc<Vec<u8>>,
        key: Arc<Vec<u8>>,
        pb: Arc<Mutex<Bar>>,
    ) {
        let self_inner = self.inner.read().await;

        let path = Path::new(&self_inner.video_id);
        let name = Self::part_name(index, self_inner.quality_index);
        let file_path = path.join(name);

        match fs::metadata(&file_path) {
            Ok(file) => {
                let size = file.len();
                let mut bar = pb.lock().await;
                bar.update(size as usize).unwrap();
                return;
            }
            Err(_) => (),
        }

        let res = self_inner.client.get(link).send().await.unwrap();

        // corrupted part
        if res.status() == 502 || res.status() == 504 {
            println!("[WARNING] Part {} of video seems to be corrupted you will experience some freezeing", index);
            let _file = fs::File::create(file_path).unwrap();
            return;
        }
        let bytes = res.bytes().await.unwrap();
        let mut bytes = bytes.to_vec();
        let cipher =
            cbc::Decryptor::<aes::Aes128>::new(key.as_slice().into(), iv.as_slice().into());

        let decrypted_bytes = cipher.decrypt_padded_mut::<Pkcs7>(&mut bytes).unwrap();

        let file = fs::File::create(&file_path);
        let mut file = match file {
            Ok(x) => x,
            Err(err) => {
                let bad_file_path = file_path.to_string_lossy();
                panic!(
                    "Cannot Open file at path {}\n{}",
                    &bad_file_path,
                    err.to_string()
                );
            }
        };

        file.write_all(decrypted_bytes).unwrap();

        let mut bar = pb.lock().await;
        bar.update(decrypted_bytes.len()).unwrap();
    }
}
