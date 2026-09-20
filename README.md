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

- Refuses path-traversal URLs, well-known secret-file paths and malformed hostnames
- Saves files to your `~/Downloads` folder by default (no clutter!)
- Supports segmented downloads for maximum speed
- Uses Rust's memory safety guarantees
- Feels like a native Linux tool with modern progress bars

Whether you're a developer downloading dependencies, a sysadmin fetching logs, or just someone who wants a safer downloader, rget gives you a clean, secure CLI for your daily downloads.

---

## ✨ Features

- 🔒 **URL safety checks** — Only `http`/`https`; refuses path traversal (`..` in any encoding), well-known secret files (`.env`, `.ssh/id_rsa`, `/etc/passwd`, …) and malformed hostnames. Ordinary URLs with `&`, `;`, `(`, `)` or `$` are fine
- 🚀 **Segmented downloads** (`--segments`) — Split files into parts and download in parallel for maximum speed
- 📦 **Parallel downloads** (`-j`) — Download multiple URLs concurrently
- 🔄 **Automatic retries** (`-r`) — Exponential backoff with jitter for transient failures (timeouts, connection errors, 408/425/429/5xx); honours the server's `Retry-After`
- 💾 **Resume support** (`-c`) — Continue interrupted downloads; the remote file is checked (`ETag`/`Last-Modified`) so a changed file restarts instead of being corrupted
- 🛡️ **Safe partial files** — data is written to `name.part` and renamed only when complete, so a failed download never damages an existing file
- 📂 **Default Downloads folder** — Files automatically save to `~/Downloads`
- ⚙️ **Configuration file** — Set defaults in `~/.config/rget/config.toml`
- 🔐 **SHA‑256 checksum verification** — Verify file integrity
- 🚦 **Rate limiting** (`--limit-rate`) — Control bandwidth usage
- 📋 **Custom headers** (`-H`) — Add authentication tokens, etc.
- 🧹 **Quiet mode** (`-q`) — Suppress all non‑error output
- 🛡️ **Blocks local and private destinations** — Refuses loopback, private, link-local (including the cloud metadata address) and reserved addresses, whether they appear in the URL, in a **redirect**, or as what a **hostname resolves to**; `--allow-private` opts out
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

> **Quote URLs that contain `&`, `;`, `(`, `)` or `$`.** Your shell interprets them before rget ever runs (an unquoted `&` sends the command to the background and cuts the URL short):
> ```bash
> rget 'https://example.com/download?id=42&format=zip'
> ```

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
While a download runs, its data lives in `large-file.iso.part` (plus a small `large-file.iso.part.meta` recording which remote file it came from). Both are removed when the download completes. Run the same command with `-c` to continue; if the remote file has changed in the meantime, rget says so and starts over. Without `-c`, a leftover `.part` file is discarded.

### Download with 4 segments (faster!)
```bash
rget --segments 4 https://example.com/large-file.iso
```
Each segment is a separate connection, retried on its own (`-r`), and stored as `large-file.iso.part0`, `.part1`, … until they are merged. If the download is interrupted, run the same command with `-c` to continue: the parts are reused only when the remote file and the segment count are unchanged, otherwise rget starts over. `--limit-rate` applies to the download as a whole.

### Download multiple URLs in parallel
```bash
rget -j 4 url1.zip url2.zip url3.zip url4.zip
```

### Add custom headers (e.g., authentication)
```bash
rget -H "Authorization: Bearer $TOKEN" https://example.com/protected.zip
```

### Download from a local or private address
```bash
rget --allow-private http://192.168.1.20:8000/backup.tar.gz
```
Loopback, private and link-local addresses (`127.0.0.1`, `192.168.x.x`, `169.254.x.x`, …) are refused by default, so a URL, a redirect or a DNS name cannot be used to make rget read something on your own machine or network. `--allow-private` (or `allow_private = true` in the config file) turns that off, for a development server or a NAS.

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
Only failures that can heal are retried (timeouts, dropped connections, and HTTP 408, 425, 429, 500, 502, 503, 504); a 404 fails immediately. A retry continues from the bytes already received, and a `Retry-After` from the server is respected (up to 60 seconds).

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
# allow_private = false   # Allow loopback, private and link-local addresses
```

**CLI arguments override config values.**

---

## 📁 Project Structure

Organised by feature: each folder under `features/` is one capability and never imports another; `app/` is the only place that wires them together.

```
rget/
├── src/
│   ├── main.rs              # thin entry point
│   ├── lib.rs               # crate root (so tests/ can use the public API)
│   ├── app/                 # CLI arguments, config file, settings, and the wiring
│   ├── features/
│   │   ├── download/        # single and segmented downloads, resume, retries, host policy
│   │   ├── validation/      # is this URL safe to fetch?
│   │   ├── integrity/       # SHA-256 verification
│   │   ├── input/           # reading URLs from a file or stdin
│   │   └── destination/     # where the file goes, and what it is called
│   └── shared/              # progress bar, sizes, network-address classification
├── tests/
│   ├── unit/                # mirrors src/
│   └── integration/         # the public API against a local HTTP server
├── benches/                 # download throughput (criterion)
├── Cargo.toml               # Dependencies and metadata
├── README.md                # This file
├── ROADMAP.md               # What is done and what is next
├── DIARY.md                 # Why things are the way they are
├── LICENSE                  # GPLv3 License
└── CHANGELOG.md             # Version history
```

---

## 🧪 Testing

```bash
# Run everything: unit, integration (against a local server) and property tests
cargo test

# Run the property tests with many more cases
PROPTEST_CASES=50000 cargo test

# Measure download throughput
cargo bench
```

---

## 🛡️ Security

rget is built with security as a priority:

- **No command injection** — Pure Rust, no shell execution
- **Memory‑safe** — No `unsafe` code
- **URL checks** — Only `http`/`https`; refuses path traversal, well-known secret-file paths and hostnames no DNS name can contain. The checks look at what a URL *means* once parsed, so legitimate URLs (`?a=1&b=2`, `file(1).zip`) are never refused
- **TLS hardening** — Uses `rustls` with modern cipher suites
- **Private address blocking** — Prevents SSRF: loopback, private, link-local and reserved addresses are refused in the URL, in redirects, and in what a name resolves to (checked at connection time, so DNS rebinding cannot swap the address). A configured system proxy does its own DNS and is outside this check
- **Safe file names** — A name taken from a URL is percent-decoded and cleaned (`/`, control characters, a leading `-`, `..`), so it can only ever name a file inside the download directory

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
