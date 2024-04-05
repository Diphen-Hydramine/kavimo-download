use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct KavimoArgs {
    /// path of a text file including links
    #[arg(long)]
    pub file: Option<String>,
    /// set timer for downloads (e.g. --timer 02:00:00-08:00:00)
    #[arg(long)]
    pub timer: Option<String>
}



impl KavimoArgs {
    pub fn validate(&self) -> bool {
        if self.timer.is_some() {
            if self.file.is_none() {
                println!("--timer is only valid if --file is specified");
                return false;
            }
        }

        true
    }
}

