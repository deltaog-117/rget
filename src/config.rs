// SPDX-License-Identifier: GPL-3.0-or-later
// rget - A safe, modern downloader for Linux
// Copyright (C) 2026  Aeon Ennoia
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.


use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    pub timeout: Option<u64>,
    pub retries: Option<u32>,
    pub user_agent: Option<String>,
    pub quiet: Option<bool>,
    pub jobs: Option<usize>,
    pub follow_redirects: Option<bool>,
    pub resume: Option<bool>,
    pub limit_rate: Option<usize>,
    pub segments: Option<usize>,
    pub directory_prefix: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        if let Some(path) = config_path {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn get_config_path() -> Option<PathBuf> {
        let base_dir = dirs::config_dir()?;
        Some(base_dir.join("rget").join("config.toml"))
    }

    #[allow(dead_code)]
    pub fn get_config_dir() -> Option<PathBuf> {
        let base_dir = dirs::config_dir()?;
        Some(base_dir.join("rget"))
    }

    #[allow(dead_code)]
    pub fn ensure_config_dir() -> Option<PathBuf> {
        let dir = Self::get_config_dir()?;
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
        }
        Some(dir)
    }

    #[allow(dead_code)]
    pub fn write_default_config() -> Result<(), Box<dyn std::error::Error>> {
        let dir = Self::ensure_config_dir().ok_or("Failed to find config directory")?;
        let path = dir.join("config.toml");
        if !path.exists() {
            let default_config = r#"# rget configuration file
# CLI arguments override these values

timeout = 30
retries = 0
# user_agent = "rget/1.0"
quiet = false
jobs = 1
follow_redirects = true
resume = false
limit_rate = 1048576  # 1 MB/s
segments = 1          # Number of parallel segments for a single file
# directory_prefix = "/path/to/downloads"
"#;
            fs::write(&path, default_config)?;
        }
        Ok(())
    }
}
