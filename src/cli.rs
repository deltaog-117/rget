use clap::Parser;

/// Parse human-readable size strings like "500k", "2M", "1G" (or "0" for no limit)
fn parse_size(s: &str) -> Result<usize, String> {
    // ... existing parse_size implementation ...
    // (I'll include it for completeness)
    let s = s.trim();
    if s == "0" {
        return Ok(0);
    }
    if s.is_empty() {
        return Err("Empty size string".to_string());
    }

    let (num_str, suffix) = {
        let chars: Vec<char> = s.chars().collect();
        let mut num_end = 0;
        for (i, c) in chars.iter().enumerate() {
            if c.is_ascii_digit() || *c == '.' {
                num_end = i + 1;
            } else {
                break;
            }
        }
        if num_end == 0 {
            return Err(format!("Invalid size format: {}", s));
        }
        let num_str: String = chars[..num_end].iter().collect();
        let suffix: String = chars[num_end..].iter().collect();
        (num_str, suffix.to_lowercase())
    };

    let num: f64 = num_str.parse().map_err(|_| format!("Invalid number: {}", num_str))?;

    let bytes = match suffix.as_str() {
        "" => num,
        "b" => num,
        "k" | "kb" => num * 1024.0,
        "m" | "mb" => num * 1024.0 * 1024.0,
        "g" | "gb" => num * 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" => num * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return Err(format!("Unknown size suffix: {}", suffix)),
    };

    if bytes < 1.0 {
        return Err("Rate limit must be at least 1 byte".to_string());
    }

    Ok(bytes as usize)
}

#[derive(Parser, Debug)]
#[command(name = "rget")]
#[command(about = "A safe, modern downloader for Linux", long_about = None)]
pub struct Args {
    /// URLs to download (can specify multiple)
    #[arg()]
    pub urls: Vec<String>,

    /// Output filename (only valid with a single URL)
    #[arg(short = 'O', long)]
    pub output: Option<String>,

    /// Output directory prefix
    #[arg(short = 'P', long)]
    pub directory_prefix: Option<String>,

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

    /// Rate limit in bytes per second (e.g., 500k, 2M, 1G). Use 0 for no limit.
    #[arg(long, value_parser = parse_size)]
    pub limit_rate: Option<usize>,

    /// Number of parallel segments for a single file (default: 1)
    #[arg(long, default_value = "1")]
    pub segments: usize,

    /// Ignore config file
    #[arg(long)]
    pub no_config: bool,

    /// Generate a default configuration file and exit
    #[arg(long)]
    pub init: bool,
}
