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

    let url = validate_url(&args.url)?;

    let output_path = if let Some(name) = args.output {
        name
    } else {
        url.path_segments()
            .and_then(|segments| segments.last())
            .filter(|&name| !name.is_empty())
            .unwrap_or("downloaded")
            .to_string()
    };

    if !args.quiet && args.verbose > 0 {
        eprintln!("🔍 Downloading: {}", url);
        eprintln!("📁 Output: {}", output_path);
        eprintln!("⏱️  Timeout: {}s", args.timeout);
        if args.resume {
            eprintln!("🔄 Resume: enabled");
        }
        if args.retries > 0 {
            eprintln!("🔁 Retries: {}", args.retries);
        }
    }

    download_file(
        &args.url,
        &output_path,
        args.resume,
        args.timeout,
        args.follow_redirects,
        args.user_agent.as_deref(),
        args.retries,
        args.quiet,
    )?;

    Ok(())
}
