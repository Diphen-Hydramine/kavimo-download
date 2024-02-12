use url::{Host, Url};
use crate::video::Video;



pub fn parse_video(input: &str) -> Result<Video, Box<dyn std::error::Error>> {
    let url = Url::parse(&input)?;
    let video_id = url.path()[1..].split('/').next().ok_or("no video Id found")?;
    let host = url.host().ok_or("no video host found")?;
    if let Host::Domain(video_host) = host {
        return Ok(Video::new(video_id.to_string(), video_host.to_string()));
    }
    Err("Cannot get video host".into())
}


