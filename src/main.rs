use std::io::stdin;
use url::Url;

mod video;
use video::Video;


#[tokio::main]
async fn main() {

    println!("Enter video iframe url: (e.g. https://stream.biomaze.ir/b6tnnbbopku1/iframe)");
    let mut user_input = String::new();
    let mut video_id = String::new();
    let mut video_host = String::new();
     
    let mut input_valid = false;
    while !input_valid {
        stdin().read_line(&mut user_input).unwrap();
        // user_input = "https://stream.biomaze.ir/b6tnnbbopku1/iframe".to_string();

        user_input = user_input.trim().to_owned();
        match Url::parse(&user_input) {
            Ok(url) => {

                match url.host() {
                    Some(host) => {
                        match host {
                            url::Host::Domain(domain) => {
                                video_host = domain.to_string();
                                let mut paths = url.path()[1..].split('/');
                                if let Some(id) = paths.next() {
                                    video_id = id.to_string();
                                    input_valid = true;
                                }
                            }
                            _ => {
                            }
                        }
                    }
                    None => () 
                }
            }
            Err(_) => {
                println!("Error parsing input please try again:");
            }
        }
    }


    println!("[Video host] {}", &video_host);
    println!("[Video id] {}", &video_id);

    let video = Video::new(video_id, video_host);

    match video.download().await {
        Ok(_) => {
        }
        Err(x) => {
            println!("Error message: '{}'", x.to_string());
        }
    };
}

