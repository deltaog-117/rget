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

//! Streams a response body to a writer through one reusable buffer.

use super::error::{Error, Result};
use super::throttle::Throttle;
use std::io::{self, Read, Write};

/// Size of the single buffer a download reuses for its whole life.
const BUFFER_SIZE: usize = 64 * 1024;

/// Copies `reader` to `writer` until end of input and returns the byte count.
///
/// Memory use is one buffer regardless of the file size. `on_chunk` is called with the
/// size of every chunk after it is written. `stall_secs` is only used to word the error
/// when the transport reports that a read timed out.
///
/// # Errors
///
/// A read failure becomes [`Error::Stalled`] (timeout), [`Error::Network`] (any other
/// transport error) or [`Error::Io`]; a write failure is always [`Error::Io`]. Pressing
/// Ctrl+C is noticed between chunks and becomes [`Error::Interrupted`], leaving whatever
/// was already written in place.
pub(super) fn copy<R, W, F>(
    reader: &mut R,
    writer: &mut W,
    throttle: &mut Throttle,
    stall_secs: u64,
    mut on_chunk: F,
) -> Result<u64>
where
    R: Read,
    W: Write,
    F: FnMut(u64),
{
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut total = 0u64;

    loop {
        if crate::shared::interrupt::requested() {
            return Err(Error::Interrupted);
        }

        let want = throttle.read_size(buffer.len());
        let read = reader
            .read(&mut buffer[..want])
            .map_err(|e| classify_read_error(e, stall_secs))?;
        if read == 0 {
            return Ok(total);
        }

        throttle.wait(read);
        writer.write_all(&buffer[..read])?;

        total += read as u64;
        on_chunk(read as u64);
    }
}

/// `reqwest` reports body failures as an `io::Error` wrapping a `reqwest::Error`; unwrap it
/// so timeouts and transport errors keep their meaning.
fn classify_read_error(error: io::Error, stall_secs: u64) -> Error {
    if !error
        .get_ref()
        .is_some_and(|inner| inner.is::<reqwest::Error>())
    {
        return Error::Io(error);
    }

    let inner = error
        .into_inner()
        .expect("checked above: the io::Error wraps a reqwest::Error");
    match inner.downcast::<reqwest::Error>() {
        Ok(e) if e.is_timeout() => Error::Stalled(stall_secs),
        Ok(e) => Error::Network(*e),
        Err(other) => Error::Io(io::Error::other(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Hands out `data` in slices whose sizes cycle through `sizes`, like a socket would.
    struct ChunkedReader {
        data: Vec<u8>,
        position: usize,
        sizes: Vec<usize>,
        turn: usize,
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let size = self.sizes[self.turn % self.sizes.len()];
            self.turn += 1;
            let n = size.min(buf.len()).min(self.data.len() - self.position);
            buf[..n].copy_from_slice(&self.data[self.position..self.position + n]);
            self.position += n;
            Ok(n)
        }
    }

    struct FailingReader(io::ErrorKind);

    impl Read for FailingReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(self.0, "boom"))
        }
    }

    struct FullDisk;

    impl Write for FullDisk {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("no space left on device"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]

        #[test]
        fn output_equals_input_for_any_data_and_chunking(
            data in proptest::collection::vec(any::<u8>(), 0..150_000),
            sizes in proptest::collection::vec(1usize..70_000, 1..8),
        ) {
            let mut reader = ChunkedReader { data: data.clone(), position: 0, sizes, turn: 0 };
            let mut out = Vec::new();
            let mut reported = 0u64;

            let total = copy(&mut reader, &mut out, &mut Throttle::new(None), 30, |n| reported += n).unwrap();

            prop_assert_eq!(&out, &data);
            prop_assert_eq!(total, data.len() as u64);
            prop_assert_eq!(reported, data.len() as u64);
        }

        #[test]
        fn read_size_never_exceeds_the_buffer_or_the_limit(limit in 1usize..10_000_000, buffer in 1usize..200_000) {
            let size = Throttle::new(Some(limit)).read_size(buffer);
            prop_assert!(size >= 1);
            prop_assert!(size <= buffer);
            prop_assert!(size <= limit);
        }
    }

    #[test]
    fn an_empty_input_writes_nothing() {
        let mut reader = ChunkedReader {
            data: vec![],
            position: 0,
            sizes: vec![10],
            turn: 0,
        };
        let mut out = Vec::new();
        assert_eq!(
            copy(&mut reader, &mut out, &mut Throttle::new(None), 30, |_| {}).unwrap(),
            0
        );
        assert!(out.is_empty());
    }

    #[test]
    fn a_read_failure_is_an_io_error() {
        let err = copy(
            &mut FailingReader(io::ErrorKind::ConnectionReset),
            &mut Vec::new(),
            &mut Throttle::new(None),
            30,
            |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn a_write_failure_is_an_io_error() {
        let mut reader = ChunkedReader {
            data: vec![1; 100],
            position: 0,
            sizes: vec![10],
            turn: 0,
        };
        let err = copy(
            &mut reader,
            &mut FullDisk,
            &mut Throttle::new(None),
            30,
            |_| {},
        )
        .unwrap_err();
        assert!(matches!(err, Error::Io(_)));
    }
}
