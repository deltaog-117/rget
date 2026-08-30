# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- (No unreleased changes yet – this is the first stable release)

---

## [0.1.0] - 2026-08-30

### Added
- **Wiki CRUD** – Full Markdown editing with Git versioning (every change committed to a local Git repository)
- **Distro Comparison Table** – Sortable, filterable, searchable table of Linux distributions (seeded with 9 major distros)
- **Full‑text Search** – MySQL FULLTEXT search with relevance ranking for wiki pages
- **Distro Family Tree** – Interactive D3.js tree visualisation showing distribution lineages
- **User Authentication** – Login, Register, Logout with Livewire and custom User model
- **"Changed Since Last Visit"** – Green badge on wiki pages updated after the user's last visit
- **Terminal Simulator** – Fully functional Linux terminal in the browser using xterm.js with a virtual filesystem and command parser
- **Light Theme with Dark Terminal** – Clean, accessible light theme for the main interface, with a dark terminal for an authentic Linux experience
- **Feature‑First Architecture** – Vertical‑slice organisation under `app/Features/`; each feature is self‑contained and can be deleted without breaking the rest
- **Shared Infrastructure** – Reusable code in `app/Shared/` (database, logging, utilities, Git service)
- **Livewire Components** – Interactive components for search bar, comparison table, terminal, and auth forms
- **Blade Views** – Clean, responsive views for all features
- **Comprehensive Documentation** – README with installation, usage, architecture overview, and screenshots
- **MIT License** – Open‑source license for the project

### Changed
- (No changes – initial release)

### Deprecated
- (None)

### Removed
- (None)

### Fixed
- (None – initial release)

### Security
- (None – initial release)
