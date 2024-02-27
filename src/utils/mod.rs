use url::{Host, Url};
use crate::video::Video;



pub fn parse_video(input: &str) -> Result<Video, Box<dyn std::error::Error>> {
    let mut splitter = input.split(' ');
    let url_text = splitter.next().ok_or("Cannot get link value from text line")?;
    let url = Url::parse(&url_text)?;
    let video_id = url.path()[1..].split('/').next().ok_or("no video Id found")?;
    let host = url.host().ok_or("no video host found")?;
    let quality = splitter.next().map(|x| x.to_owned());
    if let Host::Domain(video_host) = host {
        return Ok(Video::new(video_id.to_string(), video_host.to_string(), quality));
    }
    Err("Cannot get video host".into())
}


