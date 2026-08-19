use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rget")]
#[command(about = "A safe, modern downloader for Linux", long_about = None)]
pub struct Args {
    /// URLs to download (can specify multiple)
    #[arg(required = true)]
    pub urls: Vec<String>,

    /// Output filename (only valid with a single URL)
    #[arg(short = 'O', long)]
    pub output: Option<String>,

    /// Resume an incomplete download
    #[arg(short = 'c', long)]
    pub resume: bool,

    /// Verbose output
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Timeout in seconds (default: 30)
    #[arg(short = 't', long)]
    pub timeout: Option<u64>,

    /// Follow redirects (default: true)
    #[arg(long)]
    pub follow_redirects: Option<bool>,

    /// Custom user-agent
    #[arg(short = 'A', long)]
    pub user_agent: Option<String>,

    /// Number of retries on failure (default: 0)
    #[arg(short = 'r', long)]
    pub retries: Option<u32>,

    /// Quiet mode (no output except errors)
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// SHA‑256 checksum to verify (only with single URL)
    #[arg(long)]
    pub sha256: Option<String>,

    /// Number of parallel downloads (default: 1)
    #[arg(short = 'j', long)]
    pub jobs: Option<usize>,

    /// Ignore config file
    #[arg(long)]
    pub no_config: bool,
}
