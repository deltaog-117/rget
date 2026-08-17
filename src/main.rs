mod cli;
mod download;
mod validator;
mod progress;
mod error;

use clap::Parser;
use cli::Args;
use validator::validate_url;
use download::download_file;
use error::Result;

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    // Validate URL
    let url = validate_url(&args.url)?;

    // Determine output filename
    let output_path = if let Some(name) = args.output {
        name
    } else {
        // Extract filename from URL
        url.path_segments()
            .and_then(|segments| segments.last())
            .filter(|&name| !name.is_empty())
            .unwrap_or("downloaded")
            .to_string()
    };

    // Verbose logging
    if args.verbose > 0 {
        eprintln!("🔍 Downloading: {}", url);
        eprintln!("📁 Output: {}", output_path);
        eprintln!("⏱️  Timeout: {}s", args.timeout);
        if args.resume {
            eprintln!("🔄 Resume: enabled");
        }
    }

    // Download!
    download_file(
        &args.url,
        &output_path,
        args.resume,
        args.timeout,
        args.follow_redirects,
        args.user_agent.as_deref(),
    )?;

    Ok(())
}
