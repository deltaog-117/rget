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


use clap::Parser;
use rget::app::cli::{Args, IfExists};
use rget::app::config::Config;
use rget::app::settings::Settings;
use rget::features::destination::ExistingFile;

const URL: &str = "https://example.com/f.zip";
const HELLO: &str = "6e39426dd10db18f88f5c6b6be808b8d9f12929cd03b6ac41dba112de33ef099";

fn parse(extra: &[&str]) -> Result<Args, clap::Error> {
    Args::try_parse_from(["rget"].into_iter().chain(extra.iter().copied()).chain([URL]))
}

fn args(extra: &[&str]) -> Args {
    parse(extra).unwrap()
}

fn config(toml: &str) -> Config {
    toml::from_str(toml).unwrap()
}

fn policy(extra: &[&str], config: &Config) -> ExistingFile {
    Settings::resolve(&args(extra), config).if_exists
}

// ---- --if-exists / -n ---------------------------------------------------------------------

#[test]
fn an_existing_file_is_overwritten_unless_told_otherwise() {
    assert_eq!(policy(&[], &Config::default()), ExistingFile::Overwrite);
}

#[test]
fn the_command_line_chooses_the_policy() {
    let none = Config::default();
    assert_eq!(policy(&["--if-exists", "overwrite"], &none), ExistingFile::Overwrite);
    assert_eq!(policy(&["--if-exists", "skip"], &none), ExistingFile::Skip);
    assert_eq!(policy(&["--if-exists", "rename"], &none), ExistingFile::Rename);
    assert_eq!(policy(&["-n"], &none), ExistingFile::Skip);
    assert_eq!(policy(&["--no-clobber"], &none), ExistingFile::Skip);
}

#[test]
fn an_unknown_policy_is_a_usage_error() {
    assert!(parse(&["--if-exists", "explode"]).is_err());
}

#[test]
fn no_clobber_and_if_exists_are_mutually_exclusive() {
    assert!(parse(&["-n", "--if-exists", "rename"]).is_err());
}

#[test]
fn the_config_file_supplies_a_default_and_the_command_line_beats_it() {
    let cfg = config("if_exists = \"rename\"");
    assert_eq!(policy(&[], &cfg), ExistingFile::Rename);
    assert_eq!(policy(&["--if-exists", "skip"], &cfg), ExistingFile::Skip);
    assert_eq!(policy(&["--if-exists", "overwrite"], &cfg), ExistingFile::Overwrite);
}

#[test]
fn a_bad_value_in_the_config_file_is_rejected_by_the_parser() {
    assert!(toml::from_str::<Config>("if_exists = \"explode\"").is_err());
}

// ---- interaction with -c ------------------------------------------------------------------------

#[test]
fn resuming_together_with_skip_or_rename_on_the_command_line_is_a_conflict() {
    for extra in [&["-c", "-n"][..], &["-c", "--if-exists", "skip"], &["-c", "--if-exists", "rename"]] {
        assert!(Settings::resume_conflict(&args(extra)).is_some(), "{extra:?}");
    }
}

#[test]
fn resuming_with_overwrite_or_no_policy_is_fine() {
    assert!(Settings::resume_conflict(&args(&["-c"])).is_none());
    assert!(Settings::resume_conflict(&args(&["-c", "--if-exists", "overwrite"])).is_none());
    assert!(Settings::resume_conflict(&args(&["-n"])).is_none());
}

#[test]
fn a_skip_or_rename_default_in_the_config_does_not_apply_to_a_resume() {
    let cfg = config("if_exists = \"rename\"");
    let settings = Settings::resolve(&args(&["-c"]), &cfg);
    assert!(settings.resume);
    assert_eq!(settings.if_exists, ExistingFile::Overwrite);
}

#[test]
fn an_explicit_skip_beats_resume_set_in_the_config_file() {
    let cfg = config("resume = true");
    let settings = Settings::resolve(&args(&["-n"]), &cfg);
    assert!(!settings.resume, "the explicit command-line choice wins");
    assert_eq!(settings.if_exists, ExistingFile::Skip);
    assert!(Settings::resolve(&args(&[]), &cfg).resume);
}

#[test]
fn the_choice_is_exposed_as_the_cli_enum_too() {
    assert_eq!(args(&["--if-exists", "rename"]).if_exists, Some(IfExists::Rename));
    assert_eq!(args(&[]).if_exists, None);
}

// ---- --sha256 --------------------------------------------------------------------------------------

#[test]
fn a_digest_is_accepted_in_either_case_and_as_a_pasted_sha256sum_line() {
    for value in [HELLO.to_string(), HELLO.to_uppercase(), format!("{HELLO}  f.zip")] {
        let parsed = args(&["--sha256", &value]).sha256.unwrap();
        assert_eq!(parsed.to_hex(), HELLO, "{value}");
    }
}

#[test]
fn a_malformed_digest_is_a_usage_error_before_any_download() {
    for value in ["deadbeef", "", "not-a-digest", &"g".repeat(64)] {
        let err = parse(&["--sha256", value]).unwrap_err();
        assert!(err.to_string().contains("64 hexadecimal digits"), "{value}: {err}");
    }
}
