use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use pbkdf2::pbkdf2_hmac;
use regex::Regex;
use reqwest::{
    self,
    header::{self, HeaderMap},
    Client,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::sync::{Semaphore, OwnedSemaphorePermit};
use std::{sync::Mutex, path::Path, io::Write};
use std::sync::Arc;
use std::fs;
use kdam::{tqdm, Bar, BarExt};
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};

mod convert;
use convert::convert_video_from_mpeg_to_mp4;



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

pub struct Video {
    video_id: String,
    video_host: String,
    client: Client,
}

impl Video {
    pub fn new(video_id: String, video_host: String) -> Self {
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
            video_id,
            video_host,
            client,
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

    pub async fn download(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[Progress] fetching embed files");

        let embed_url = format!("https://{}/{}/embed", &self.video_host, &self.video_id);

        let embed_res = self.client.get(&embed_url).send().await?;
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
                return Err("".into());
            }
        }

        println!("[Progress] Fetching playlists");

        let playlist_url = format!(
            "https://{}/{}.m3u8",
            &self.video_host, &embed_video_data.playlist
        );

        let playlist_res = self.client.get(playlist_url).send().await?;
        if playlist_res.status() != 200 {
            println!("[Error] Cannot get playlist file");
            return Err("".into());
        }

        let encrypted_playlist_text = playlist_res.text().await?;

        let playlist_text = Self::decrypt_m3u8(&embed_video_data.msgn, &encrypted_playlist_text)?;
        println!("[Prompt] Select desired quality: ");
        for (index, video_quality) in embed_video_data.download.iter().enumerate() {
            println!("[Choice] {} -> {}", video_quality.name, index);
        }

        let mut index_string = String::new();
        let mut valid_selection = false;
        let mut selected_playlist_link = "";
        let mut q_index = 0;
        while !valid_selection {
            std::io::stdin().read_line(&mut index_string)?;
            index_string = index_string.trim().to_string();
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
                    println!("[Error] Cannot parse input to u8 try again:");
                }
            }
        }

        let playlist_m3u8_res = self.client.get(selected_playlist_link).send().await?;
        if playlist_m3u8_res.status() != 200 {
            println!("[Error] Cannot get playlist parts");
            return Err("".into());
        }

        let encrypted_playlist_text = playlist_m3u8_res.text().await?;
        let playlist_text = Self::decrypt_m3u8(&embed_video_data.msgn, &encrypted_playlist_text)?;
        let lines = playlist_text.split('\n');

        let mut part_links = Vec::new();
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
                        let res = self.client.get(link).send().await?;
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
                part_links.push(line);
            }
        }


        let download_semaphore = Arc::new(Semaphore::new(10));
        let mut download_handles = Vec::new();

        let mut safe_title = embed_video_data.title;
        for char in  r#"\/:*?"<>|"#.chars() {
            safe_title = safe_title.replace(char, "-");
        }

        let _ = fs::create_dir(&safe_title);
        let arc_cipher_key = Arc::new(cipher_key);
        let arc_cipher_iv = Arc::new(cipher_iv);
        let arc_safe_title = Arc::new(safe_title.to_string());

        let total_size = embed_video_data.download[q_index].size.parse::<usize>()?;
        let pb = tqdm!(
            total = total_size,
            unit_scale = true,
            unit_divisor = 1024,
            unit = "B"
        );

        let pb = Arc::new(Mutex::new(pb));
        
        for (index, link) in part_links.iter().enumerate() {
            let semaphore = download_semaphore.clone();
            let permit = semaphore.acquire_owned().await?;
            let client = self.client.clone();
            let iv = arc_cipher_iv.clone();
            let key = arc_cipher_key.clone();
            let pb = pb.clone();
            let safe_title = arc_safe_title.clone();
            let fut = Self::download_part(index, link.to_string(), client, permit, iv, key, q_index, pb, safe_title);
            let handle = tokio::spawn(fut);
            download_handles.push(handle);
        }        

        for handle in download_handles {
            handle.await?;
        }


        let directory_path = Path::new(&safe_title);
        let mut outfile = fs::File::create(directory_path.join("placeholder.mpeg"))?;
        for index in 0..part_links.len() {
            let name = format!("Vpart-{:010}-{}", index, q_index);
            let file_content = fs::read(directory_path.join(name))?;
            outfile.write_all(&file_content)?;
        }

        println!("[Progress] Created mpeg video");

        let input_bytes = directory_path.join("placeholder.mpeg");
        let input_bytes = input_bytes.to_string_lossy();
        let input_bytes = input_bytes.as_bytes();
        let mut input_bytes_vec = Vec::with_capacity(input_bytes.len() + 1);
        input_bytes_vec.extend_from_slice(&input_bytes);
        input_bytes_vec.push(0b0u8);

        let output_bytes = format!("{}.mp4", &safe_title);
        let output_bytes = output_bytes.as_bytes();
        let mut output_bytes_vec = Vec::with_capacity(output_bytes.len() + 1);
        output_bytes_vec.extend_from_slice(&output_bytes);
        output_bytes_vec.push(0b0u8);

        unsafe {
            convert_video_from_mpeg_to_mp4(
                input_bytes_vec.as_ptr() as *const libc::c_char,
                output_bytes_vec.as_ptr() as *const libc::c_char
            );
        }

        println!("[Progress] mp4 video created");

        fs::remove_dir_all(directory_path)?;

        println!("[Progress] directory deleted");

        Ok(())
    }

    async fn download_part(index: usize, link: String, client: Client, _permit: OwnedSemaphorePermit, iv: Arc<Vec<u8>>, key: Arc<Vec<u8>>, q_index: usize, pb: Arc<Mutex<Bar>>, safe_title: Arc<String>) {

        let path = Path::new(safe_title.as_str());
        let name = format!("Vpart-{:010}-{}", index, q_index);
        let file_path = path.join(name);

        match fs::metadata(&file_path) {
            Ok(file) => {
                let size = file.len();
                let mut bar = pb.lock().unwrap();
                bar.update(size as usize).unwrap();
                return ;
            }
            Err(_) => ()
        }

        let bytes = client.get(link).send().await.unwrap().bytes().await.unwrap();
        let mut bytes = bytes.to_vec();
        let cipher = cbc::Decryptor::<aes::Aes128>::new(key.as_slice().into(), iv.as_slice().into());

        let decrypted_bytes = cipher.decrypt_padded_mut::<Pkcs7>(&mut bytes).unwrap();

        let mut file = fs::File::create(file_path).unwrap();
        file.write_all(decrypted_bytes).unwrap();

        let mut bar = pb.lock().unwrap();
        bar.update(decrypted_bytes.len()).unwrap();

    }
}
