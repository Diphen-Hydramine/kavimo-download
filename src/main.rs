use std::fs::read_to_string;
use std::io::stdin;
use clap::Parser as _;

mod video;
mod arguments;
mod timer;
mod utils;
use utils::*;

use crate::timer::TimedDownload as _;


#[tokio::main]
async fn main() {
    let args = arguments::KavimoArgs::parse();
    if !args.validate() {
        return ;
    }

    if let Some(batch_file) = args.file {
        match read_to_string(&batch_file) {
            Ok(file_content) => {
                let mut videos = Vec::new();
                let time_range = match args.timer {
                    Some(x) => {
                        match timer::parse_time(&x) {
                            Ok(time_range) => Some(time_range),
                            Err(_) => {
                                println!("[ERROR] '{}' is not a valid timer", &x);
                                return ;
                            }
                        }
                    }
                    None => None
                };
                for line in file_content.lines() {
                    if let Ok(video) = parse_video(line) {
                        videos.push(video);
                    } else {
                        println!("[ERROR] '{}' is not a valid link", line);
                    }
                }
                println!("[Progress] Parsed all videos, count: {}", videos.len());
                println!("[Progress] Starting download");
                for mut video in videos {
                    if let Some(ref timer) = time_range {
                        video.set_time_range(timer.clone()).await;
                    }
                    if !time_range.should_coutinue() {
                        println!("[INFO] Exited the time range specified stopping the program");
                        return ;
                    }
                    match video.download(true).await {
                        Ok(_) => (),
                        Err(x) => {
                            println!("[ERROR] Error message: '{}'", x.to_string());
                        }
                    }
                }
            }
            Err(err) => {
                println!(
                    "Cannot open input file: '{}' due {}",
                    &batch_file,
                    err.to_string()
                );
            }
        }

        std::process::exit(0);
    }

    println!("Enter video iframe url: (e.g. https://stream.kavimo.com/chn2rbqavgjt/embed)");
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

                video.print_extracted().await;

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
