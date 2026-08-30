# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- (None yet – this is the initial stable release)

---

## [1.0.0] - 2026-08-30

### Added

#### Core Download Features
- HTTP/HTTPS downloads with modern progress bar (ETA, speed, percentage)
- Resume support for interrupted downloads (`-c`, `--resume`)
- Custom output filename (`-O`, `--output`)
- Custom output directory (`-P`, `--directory-prefix`)
- **Default Downloads folder** – files automatically save to `~/Downloads` when no `-P` or `-O` is specified
- Verbose mode for debugging (`-v`, `--verbose`)
- Timeout control for connection and read operations (`-t`, `--timeout`)
- Custom User-Agent header (`-A`, `--user-agent`)
- Custom HTTP headers (`-H`, `--header`) – can be used multiple times
- Follow redirects (enabled by default) with option to disable (`--follow-redirects`)

#### Performance Features
- **Segmented parallel downloads** (`--segments`) – split single file into parts and download concurrently for maximum speed
- **Parallel downloads** (`-j`, `--jobs`) – download multiple URLs concurrently
- **Rate limiting** (`--limit-rate`) – throttle bandwidth usage (supports human-readable sizes: `500k`, `2M`, `1G`)
- Connection pooling via reqwest for optimal performance

#### Reliability Features
- **Automatic retries** (`-r`, `--retries`) – exponential backoff with jitter for unstable networks
- **Resume support** – automatically resumes interrupted downloads when `-c` is used
- **SHA‑256 checksum verification** (`--sha256`) – verify file integrity after download
- Timeout handling with configurable limits

#### Security Features
- **Military‑grade input sanitization** – custom implementation that blocks:
  - Command injection (`;`, `|`, `&`, `$`, `` ` ``, `(`, `)`, `<`, `>`)
  - Path traversal (`../`, `..\\`)
  - Sensitive file patterns (`/etc/passwd`, `/etc/shadow`, `.env`, `.git/config`, `.aws/credentials`, `.ssh/id_rsa`)
  - Encoded injection attacks (URL‑decoded and re‑checked)
- **Private IP address blocking** – prevents SSRF attacks:
  - IPv4: `localhost`, `127.0.0.1`, `192.168.x.x`, `10.x.x.x`, `172.16-31.x.x`
  - IPv6: `::1` (loopback), `fc00::/7` (unique local), `fe80::/10` (link‑local)
- **TLS hardening** – uses rustls with modern cipher suites (no OpenSSL)
- **Memory safety** – zero `unsafe` code, pure Rust
- **No shell execution** – prevents command injection entirely

#### Usability Features
- **Configuration file** (`~/.config/rget/config.toml`) – set defaults for all options
- **`--init` flag** – generates a default configuration file with comments
- **`--version` flag** – displays version information
- **Batch downloads** (`-i`, `--input-file`) – read URLs from a file
- **Stdin support** (`-i -`) – pipe URLs from other commands
- **Quiet mode** (`-q`, `--quiet`) – suppress all non‑error output
- **Colored, modern progress bars** with ETA and transfer speed
- **Help text** – comprehensive `--help` output via clap

#### Documentation
- Professional `README.md` with:
  - Feature list with emoji icons
  - Installation instructions (Cargo, source, binary)
  - Quick start examples
  - Configuration guide
  - Security comparison table (rget vs curl vs wget)
  - Project structure overview
  - Contributing guidelines link
  - License information (GPLv3)
- `CHANGELOG.md` – this file
- `LICENSE` – GNU General Public License v3.0

### Changed
- (None – this is the initial release)

### Deprecated
- (None – this is the initial release)

### Removed
- (None – this is the initial release)

### Fixed
- (None – this is the initial release)

### Security
- Built‑in command injection protection
- Built‑in path traversal protection
- Private IP address blocking (IPv4 + IPv6)
- Sensitive file pattern blocking
- TLS hardening with rustls
- Memory safety guarantees via Rust
- No shell execution – eliminates entire class of vulnerabilities
- URL sanitization before any processing occurs
