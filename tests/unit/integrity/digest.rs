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


use proptest::prelude::*;
use rget::features::integrity::{Error, Sha256Digest};

const HELLO: &str = "6e39426dd10db18f88f5c6b6be808b8d9f12929cd03b6ac41dba112de33ef099";

#[test]
fn sixty_four_hex_digits_parse_in_either_case() {
    let lower = Sha256Digest::parse(HELLO).unwrap();
    let upper = Sha256Digest::parse(&HELLO.to_uppercase()).unwrap();
    let mixed = Sha256Digest::parse(&format!("{}{}", HELLO[..32].to_uppercase(), &HELLO[32..])).unwrap();
    assert_eq!(lower, upper);
    assert_eq!(lower, mixed);
    assert_eq!(lower.to_hex(), HELLO);
    assert_eq!(upper.to_string(), HELLO, "the canonical form is lower case");
}

#[test]
fn only_the_first_word_is_read_so_sha256sum_output_can_be_pasted() {
    for line in [format!("{HELLO}  file.zip"), format!("  {HELLO}\n"), format!("{HELLO} *file.zip\n"), format!("{HELLO}\tname with spaces.bin")] {
        assert_eq!(Sha256Digest::parse(&line).unwrap().to_hex(), HELLO, "{line:?}");
    }
}

#[test]
fn anything_else_is_refused_with_the_offending_text() {
    for bad in ["", "   ", "deadbeef", "0x6e39", &HELLO[..63], &format!("{HELLO}0"), &HELLO.replace('6', "g"), &"é".repeat(32), &format!("z{}", &HELLO[1..])] {
        match Sha256Digest::parse(bad) {
            Err(Error::InvalidDigest(text)) => assert!(text.len() <= 80 * 4, "{text:?}"),
            other => panic!("{bad:?} should be refused, got {other:?}"),
        }
    }
}

#[test]
fn a_very_long_input_is_shortened_in_the_message() {
    match Sha256Digest::parse(&"a".repeat(10_000)) {
        Err(Error::InvalidDigest(text)) => assert_eq!(text.len(), 80),
        other => panic!("{other:?}"),
    }
}

proptest! {
    #[test]
    fn any_32_bytes_survive_a_round_trip_through_either_case(bytes in proptest::array::uniform32(any::<u8>())) {
        let digest = Sha256Digest::from_bytes(bytes);
        let hex = digest.to_hex();
        prop_assert_eq!(hex.len(), 64);
        prop_assert_eq!(Sha256Digest::parse(&hex).unwrap(), digest.clone());
        prop_assert_eq!(Sha256Digest::parse(&hex.to_uppercase()).unwrap(), digest);
    }

    #[test]
    fn text_of_any_other_length_is_never_accepted(text in "[0-9a-fA-F]{0,200}") {
        prop_assume!(text.len() != 64);
        prop_assert!(Sha256Digest::parse(&text).is_err());
    }

    #[test]
    fn parsing_never_panics(text in any::<String>()) {
        let _ = Sha256Digest::parse(&text);
    }
}
