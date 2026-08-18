use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rget")]
#[command(about = "A safe, modern downloader for Linux", long_about = None)]
pub struct Args {
    pub url: String,
    #[arg(short = 'O', long)]
    pub output: Option<String>,
    #[arg(short = 'c', long)]
    pub resume: bool,
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    #[arg(short = 't', long, default_value = "30")]
    pub timeout: u64,
    #[arg(long, default_value = "true", action = clap::ArgAction::Set, value_parser = clap::value_parser!(bool))]
    pub follow_redirects: bool,
    #[arg(short = 'A', long)]
    pub user_agent: Option<String>,
    #[arg(short = 'r', long, default_value = "0")]
    pub retries: u32,
}
