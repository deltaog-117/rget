Here's a complete project specification you can save for later:

---

# Project: `rsget` — A Safe, Modern Downloader for Linux

## 🎯 Vision

A memory-safe, self-contained replacement for `wget` and `curl` for everyday HTTP/HTTPS downloads, written in Rust. Focuses on security, modern ergonomics, and ease of use—not protocol breadth.

---

## 📋 Core Specifications

### Language & Ecosystem
| Aspect | Choice |
| :--- | :--- |
| **Language** | Rust (latest stable) |
| **HTTP Client** | `reqwest` with `rustls` (no OpenSSL) |
| **Async Runtime** | `tokio` (for concurrent downloads) |
| **CLI Parsing** | `clap` with derive macros |
| **Progress Bars** | `indicatif` |
| **Error Handling** | `anyhow` or `thiserror` |
| **Logging** | `env_logger` or `tracing` |
| **Target Platforms** | Linux (primary), with cross-platform optional |

### Feature Set (MVP ~500-800 lines)

| Feature | Status | Notes |
| :--- | :--- | :--- |
| HTTP/HTTPS GET | ✅ Core | Supports redirects, custom headers |
| Resume Downloads | ✅ | `Range:` header support |
| Progress Bar | ✅ | Speed, ETA, percentage |
| Output to File | ✅ | Custom filename or auto-detect |
| Timeout Configuration | ✅ | Connection and read timeouts |
| User-Agent Setting | ✅ | Customizable |
| Verbose Mode | ✅ | Debug output |
| URL Validation | ✅ | Block internal IPs, invalid schemes |
| Follow Redirects | ✅ | Configurable limit |

### Extended Features (Phase 2 ~1,500-2,500 lines)

| Feature | Status | Notes |
| :--- | :--- | :--- |
| Parallel Downloads | 🔄 Planned | Multiple URLs concurrently |
| Checksum Verification | 🔄 Planned | SHA256, MD5 via CLI flags |
| Retry Logic | 🔄 Planned | Exponential backoff |
| Cookie Support | 🔄 Planned | Persistent cookie jar |
| Rate Limiting | 🔄 Planned | Bandwidth throttling |
| Output to stdout | 🔄 Planned | For piping to other commands |
| Quiet Mode | 🔄 Planned | No output except errors |
| Configuration File | 🔄 Planned | TOML-based defaults |

### Future Expansion (Phase 3 ~3,000-5,000 lines)

| Feature | Status | Notes |
| :--- | :--- | :--- |
| SFTP Support | 🗓️ Roadmap | Via `remotefs` or `russh` |
| SCP Support | 🗓️ Roadmap | Via `remotefs` or `russh` |
| FTP Support | 🗓️ Roadmap | Via `suppaftp` |
| Recursive Mirroring | 🗓️ Roadmap | HTML link parsing with `scraper` |
| Download Queues | 🗓️ Roadmap | Batch processing from file |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│                     rsget CLI                       │
│              (clap argument parsing)                │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│                 Input Validator                      │
│  - URL sanitization (huginn)                        │
│  - Block internal IPs / dangerous patterns          │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│              Download Manager                        │
│  - reqwest client with rustls                       │
│  - Connection pooling                               │
│  - Timeout management                               │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│              Stream Processor                        │
│  - Chunked writing to disk                          │
│  - Resume support (Range headers)                   │
│  - Progress reporting (indicatif)                   │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│              Post-Processing                         │
│  - Checksum verification                            │
│  - File permission setting                          │
└─────────────────────────────────────────────────────┘
```

---

## 🛡️ Security Features

| Layer | Implementation |
| :--- | :--- |
| **No Shell Injection** | `std::process` not used—pure Rust |
| **TLS Hardening** | Force TLS 1.2/1.3 via `rustls` |
| **URL Validation** | Block localhost, private IPs, invalid schemes |
| **Memory Safety** | Entirely safe Rust—no `unsafe` code |
| **Capability Dropping** | Optional via `caps` crate (Linux) |
| **Sandboxing** | Optional via `landlock` (Linux) |

---

## 📁 Project Structure

```
rsget/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs          # CLI entry point
│   ├── cli.rs           # Argument parsing
│   ├── download.rs      # Core download logic
│   ├── validator.rs     # URL/input validation
│   ├── progress.rs      # Progress bar
│   ├── resume.rs        # Resume logic
│   ├── checksum.rs      # Hash verification
│   └── error.rs         # Custom error types
├── tests/
│   ├── integration.rs
│   └── unit/
└── examples/
    └── basic_download.rs
```

---

## 📦 Dependencies (Initial)

```toml
[dependencies]
reqwest = { version = "0.12", features = ["rustls-tls", "stream", "json"] }
clap = { version = "4.5", features = ["derive"] }
indicatif = "0.17"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
thiserror = "1.0"
env_logger = "0.11"
url = "2.5"
huginn = "0.1"  # For sanitization
```

---

## 🎯 Success Criteria

**MVP Done when:**
1. `rsget https://example.com/file.zip` downloads successfully
2. `rsget -O output.zip https://example.com/file.zip` saves with custom name
3. Resume works with `-c` flag
4. Progress bar shows speed/ETA
5. Fails safely on invalid URLs
6. No shell injection possible

**Phase 2 Done when:**
7. Parallel downloads work (`-j 4`)
8. Checksum verification works (`--sha256`)
9. Retry logic works (`--retries 3`)
10. Cookies persist across requests

---

## ⏱️ Estimated Effort

| Phase | Lines | Time (estimate) |
| :--- | :--- | :--- |
| MVP | ~500-800 | 1-2 weekends |
| Phase 2 | ~1,500-2,500 | 2-3 weekends |
| Phase 3 | ~3,000-5,000 | 1-2 months |

---

## 🚀 Why Build This?

- **Safer** than `curl`/`wget` (no command injection, memory-safe)
- **Self-contained** (no external dependencies)
- **Modern** (async, progress bars, colored output)
- **Extensible** (add SFTP/SCP later)
- **Learning** (great way to master Rust, HTTP, and systems programming)

---

Keep this spec handy. When you're ready, start with the MVP—it'll be a useful tool in under 1,000 lines. Good luck, and ping me when you begin! 🦀
