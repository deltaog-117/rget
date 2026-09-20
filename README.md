# ⚡ rget — A Safe, Modern Downloader for Linux

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.96%2B-orange.svg)](https://www.rust-lang.org/)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](http://makeapullrequest.com)
[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://github.com/YOUR_USERNAME/rget/releases)

**A memory‑safe, self‑contained replacement for `wget` and `curl` for everyday HTTP/HTTPS downloads, written in Rust.**

---

## 💡 About

Every time I needed to download a file from the terminal, I found myself torn between `curl` (powerful but verbose) and `wget` (simple but limited). Both are written in C, which means they're vulnerable to memory corruption bugs and command injection attacks.

I wanted a tool that combined the **best of both** — `curl`'s flexibility with `wget`'s simplicity — while being **secure by default**. So I built **rget**: a modern, safe downloader that:

- Blocks command injection and path traversal attacks
- Saves files to your `~/Downloads` folder by default (no clutter!)
- Supports segmented downloads for maximum speed
- Uses Rust's memory safety guarantees
- Feels like a native Linux tool with modern progress bars

Whether you're a developer downloading dependencies, a sysadmin fetching logs, or just someone who wants a safer downloader, rget gives you a clean, secure CLI for your daily downloads.

---

## ✨ Features

- 🔒 **Military‑grade sanitization** — Blocks command injection, path traversal, and sensitive file access
- 🚀 **Segmented downloads** (`--segments`) — Split files into parts and download in parallel for maximum speed
- 📦 **Parallel downloads** (`-j`) — Download multiple URLs concurrently
- 🔄 **Automatic retries** (`-r`) — Exponential backoff with jitter for unstable networks
- 💾 **Resume support** (`-c`) — Resume interrupted downloads
- 📂 **Default Downloads folder** — Files automatically save to `~/Downloads`
- ⚙️ **Configuration file** — Set defaults in `~/.config/rget/config.toml`
- 🔐 **SHA‑256 checksum verification** — Verify file integrity
- 🚦 **Rate limiting** (`--limit-rate`) — Control bandwidth usage
- 📋 **Custom headers** (`-H`) — Add authentication tokens, etc.
- 🧹 **Quiet mode** (`-q`) — Suppress all non‑error output
- 🛡️ **Block localhost & private IPs** — Prevents SSRF attacks
- 🔁 **Batch downloads** (`-i`) — Download from a file or stdin

---

## 📋 Requirements

- **Rust** 1.96 or higher (for building from source)
- **Linux** (primary target; macOS and Windows may work but are untested)
- **No external dependencies** — rget is fully self‑contained

---

## 📦 Installation

### Option 1: Install via Cargo (recommended)
```bash
cargo install rget
```

### Option 2: Build from source
```bash
git clone https://github.com/YOUR_USERNAME/rget.git
cd rget
cargo build --release
sudo cp target/release/rget /usr/local/bin/
```

### Option 3: Binary download (if available)
```bash
# Download the latest release from GitHub
curl -LO https://github.com/YOUR_USERNAME/rget/releases/latest/download/rget
chmod +x rget
sudo mv rget /usr/local/bin/
```

---

## 🚀 Quick Start

### Basic download
```bash
rget https://example.com/file.zip
```
**File saves to:** `~/Downloads/file.zip`

### Custom output name
```bash
rget -O custom.zip https://example.com/file.zip
```

### Custom output directory
```bash
rget -P ~/Documents https://example.com/file.zip
```

### Resume an interrupted download
```bash
rget -c https://example.com/large-file.iso
```

### Download with 4 segments (faster!)
```bash
rget --segments 4 https://example.com/large-file.iso
```

### Download multiple URLs in parallel
```bash
rget -j 4 url1.zip url2.zip url3.zip url4.zip
```

### Add custom headers (e.g., authentication)
```bash
rget -H "Authorization: Bearer $TOKEN" https://example.com/protected.zip
```

### Batch download from a file
```bash
# Create a file with one URL per line
echo "https://example.com/file1.zip" > urls.txt
echo "https://example.com/file2.zip" >> urls.txt
rget -i urls.txt
```

### Batch download from stdin
```bash
cat urls.txt | rget -i -
```

### Rate limit to 1 MB/s
```bash
rget --limit-rate 1M https://example.com/large-file.iso
```

### Retry up to 3 times on failure
```bash
rget -r 3 https://example.com/unstable-file.zip
```

### Verify SHA‑256 checksum
```bash
rget --sha256 abc123def... https://example.com/file.zip
```

### Quiet mode (no output except errors)
```bash
rget -q https://example.com/file.zip
```

### Generate default config file
```bash
rget --init
```

### Show version
```bash
rget --version
```

---

## ⚙️ Configuration

Create `~/.config/rget/config.toml` with defaults:

```toml
# rget configuration file
# CLI arguments override these values

timeout = 30              # Connect timeout and longest pause in the data, in seconds
retries = 0               # Number of retries on failure
# user_agent = "rget/1.0" # Custom user-agent (uncomment to set)
quiet = false             # Quiet mode (suppress progress bars)
jobs = 1                  # Parallel downloads
follow_redirects = true   # Follow redirects
resume = false            # Resume downloads by default
limit_rate = 1048576      # 1 MB/s rate limit
segments = 1              # Number of parallel segments for a single file
# directory_prefix = "/path/to/downloads"  # Default output directory
```

**CLI arguments override config values.**

---

## 📁 Project Structure

```
rget/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── cli.rs           # Argument parsing
│   ├── download.rs      # Core download logic
│   ├── validator.rs     # URL/input validation
│   ├── progress.rs      # Progress bar
│   ├── error.rs         # Custom error types
│   ├── checksum.rs      # Hash verification
│   └── config.rs        # Configuration file handling
├── docs/
│   └── rget.1           # Unix man page
├── Cargo.toml           # Dependencies and metadata
├── README.md            # This file
├── LICENSE              # GPLv3 License
├── CHANGELOG.md         # Version history
└── CONTRIBUTING.md      # Contribution guidelines
```

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_download
```

---

## 🛡️ Security

rget is built with security as a priority:

- **No command injection** — Pure Rust, no shell execution
- **Memory‑safe** — No `unsafe` code
- **Input sanitization** — Blocks dangerous characters, path traversal, and sensitive file access
- **TLS hardening** — Uses `rustls` with modern cipher suites
- **Private IP blocking** — Prevents SSRF attacks

### Security Comparison

| Feature | rget | curl | wget |
|:---|:---|:---|:---|
| Memory safety | ✅ (Rust) | ❌ (C) | ❌ (C) |
| Command injection protection | ✅ | ❌ | ❌ |
| Built‑in sanitization | ✅ | ❌ | ❌ |
| Private IP blocking | ✅ | ❌ | ❌ |
| TLS hardening | ✅ (rustls) | ⚠️ (OpenSSL) | ⚠️ (OpenSSL) |

---

## 🤝 Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/amazing`).
3. Commit your changes (`git commit -m 'Add amazing feature'`).
4. Push to the branch (`git push origin feature/amazing`).
5. Open a Pull Request.

Read the [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

---

## 📄 License

This project is licensed under the **GNU General Public License v3.0** — see the [LICENSE](LICENSE) file for details.

**Why GPL?** This license ensures that `rget` and all its derivatives remain free and open-source for everyone. Any modifications or derivative works must also be released under GPL when distributed. This protects the freedom of the software and its users.

---

## 🙏 Acknowledgements

- [reqwest](https://docs.rs/reqwest/) — HTTP client library
- [clap](https://docs.rs/clap/) — CLI argument parsing
- [indicatif](https://docs.rs/indicatif/) — Progress bars
- [rustls](https://docs.rs/rustls/) — Modern TLS implementation
- The entire Rust community for their incredible tools and inspiration

---

## 💬 Questions / Support

Open an [issue](https://github.com/YOUR_USERNAME/rget/issues) or reach out via [GitHub Discussions](https://github.com/YOUR_USERNAME/rget/discussions).

---

## 📜 Changelog

See the [CHANGELOG.md](CHANGELOG.md) file for version history.
