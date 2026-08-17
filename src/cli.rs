use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rget")]
#[command(about = "A safe, modern downloader for Linux", long_about = None)]
pub struct Args {
    /// URL to download
    pub url: String,

    /// Output filename (optional)
    #[arg(short = 'O', long)]
    pub output: Option<String>,

    /// Resume an incomplete download
    #[arg(short = 'c', long)]
    pub resume: bool,

    /// Verbose output
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Timeout in seconds (default: 30)
    #[arg(short = 't', long, default_value = "30")]
    pub timeout: u64,

    /// Follow redirects (default: true)
    #[arg(long, default_value = "true", action = clap::ArgAction::Set, value_parser = clap::value_parser!(bool))]
    pub follow_redirects: bool,

    /// Custom user-agent
    #[arg(short = 'A', long)]
    pub user_agent: Option<String>,
}
