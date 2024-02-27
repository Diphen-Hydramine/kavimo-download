use std::{fs::read_to_string, io::stdin};
use clap::Parser;

mod video;

mod utils;
use utils::parse_video;


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    file: Option<String>,
}

#[tokio::main]
async fn main() {

    let args = Args::parse();

    if let Some(batch_file) = args.file {
        match  read_to_string(&batch_file) {
            Ok(file_content) => {
                let mut videos = Vec::new();
                for line in file_content.lines() {
                    dbg!(line);
                    if let Ok(video) = parse_video(line) {
                        videos.push(video);
                    } else {
                        println!("[ERROR] '{}' is not a valid link", line);
                    }
                }
                println!("[Progress] Parsed all videos, count: {}", videos.len());
                println!("[Progress] Starting download");
                for video in videos {
                    match video.download(true).await {
                        Ok(_) => (),
                        Err(x) => {
                            println!("[ERROR] Error message: '{}'", x.to_string());
                        }
                    }
                }
            }
            Err(err) => {
                println!("Cannot open input file: '{}' due {}", &batch_file, err.to_string());
            } 
        }

        std::process::exit(0);
    } 

    println!("Enter video iframe url: (e.g. https://stream.biomaze.ir/b6tnnbbopku1/iframe)");
    let mut user_input = String::new(); 
    let mut input_valid = false;

    while !input_valid {
        user_input.clear();
        stdin().read_line(&mut user_input).unwrap();
        // user_input = "https://stream.biomaze.ir/b6tnnbbopku1/iframe".to_string();
        user_input = user_input.trim().to_owned();

        match parse_video(&user_input) {
            Ok(video) => {
                input_valid = true;

                println!("[Video host] {}", &video.video_host);
                println!("[Video id] {}", &video.video_id);

                match video.download(false).await {
                    Ok(_) => {
                        std::process::exit(0);
                    }
                    Err(x) => {
                        println!("[ERROR] Error message: '{}'", x.to_string());
                    }
                };
            }
            Err(_) => {
                println!("Cannot parse data from url provided try again: ");
            }
        }
        
    }


}


