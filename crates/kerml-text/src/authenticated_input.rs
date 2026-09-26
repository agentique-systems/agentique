//! Bound and hash the exact bytes consumed by a decoder, including its EOF probe.
//! This authenticates transport only; accepted graph/context checks stay separate.
use sha2::{Digest, Sha256};
use std::io::{self, Read};

pub(crate) struct AuthenticatedInput<R> {
    inner: R,
    expected: u64,
    remaining: u64,
    bytes: u64,
    digest: Sha256,
    eof: bool,
    failed: bool,
}

impl<R: Read> AuthenticatedInput<R> {
    pub(crate) fn new(inner: R, expected: u64) -> io::Result<Self> {
        let remaining = expected
            .checked_add(1)
            .ok_or_else(|| invalid("authenticated input size overflow"))?;
        Ok(Self {
            inner,
            expected,
            remaining,
            bytes: 0,
            digest: Sha256::new(),
            eof: false,
            failed: false,
        })
    }

    /// Never drain a decoder's unread suffix: successful parsing must itself
    /// observe real EOF, without short, excess or failed input.
    pub(crate) fn finish(self) -> io::Result<[u8; 32]> {
        if self.failed || !self.eof || self.bytes != self.expected {
            return Err(invalid(
                "authenticated input byte count, EOF or I/O failure",
            ));
        }
        Ok(self.digest.finalize().into())
    }
}

impl<R: Read> Read for AuthenticatedInput<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.failed {
            return Err(invalid("authenticated input previously failed"));
        }
        // Read's empty-buffer result says nothing about the underlying EOF.
        if buffer.is_empty() || self.eof {
            return Ok(0);
        }
        let limit = self.remaining.min(buffer.len() as u64) as usize;
        let count = match self.inner.read(&mut buffer[..limit]) {
            Ok(count) => count,
            Err(error) => {
                if error.kind() != io::ErrorKind::Interrupted {
                    self.failed = true;
                }
                return Err(error);
            }
        };
        if count == 0 {
            self.eof = true;
        } else {
            self.remaining -= count as u64;
            self.bytes += count as u64;
            if self.bytes > self.expected {
                self.failed = true;
                return Err(invalid("authenticated input exceeds trusted byte count"));
            }
            self.digest.update(&buffer[..count]);
        }
        Ok(count)
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, ErrorKind};

    #[test]
    fn exact_digest_survives_short_reads_and_interruption() {
        struct Chunks<'a>(&'a [u8], usize);
        impl Read for Chunks<'_> {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                self.1 += 1;
                if self.1.is_multiple_of(4) {
                    return Err(io::Error::from(ErrorKind::Interrupted));
                }
                let limit = (self.1 % 7 + 1).min(buffer.len());
                self.0.read(&mut buffer[..limit])
            }
        }
        let bytes = b"exact bytes including CRLF\r\n and final LF\n";
        let mut input = AuthenticatedInput::new(Chunks(bytes, 0), bytes.len() as u64).unwrap();
        let mut decoded = Vec::new();
        input.read_to_end(&mut decoded).unwrap();
        assert_eq!(decoded, bytes);
        assert_eq!(
            input.finish().unwrap(),
            <[u8; 32]>::from(Sha256::digest(bytes))
        );
    }

    #[test]
    fn requires_real_eof_and_exact_size_even_after_empty_read() {
        let mut input = AuthenticatedInput::new(Cursor::new(b"abc"), 3).unwrap();
        input.read_exact(&mut [0; 3]).unwrap();
        assert_eq!(input.read(&mut []).unwrap(), 0);
        assert!(input.finish().is_err());
        let input = AuthenticatedInput::new(Cursor::new(b""), 0).unwrap();
        assert!(input.finish().is_err());
        let mut input = AuthenticatedInput::new(Cursor::new(b""), 0).unwrap();
        assert_eq!(input.read(&mut [0; 1]).unwrap(), 0);
        assert_eq!(
            input.finish().unwrap(),
            <[u8; 32]>::from(Sha256::digest(b""))
        );
        assert!(AuthenticatedInput::new(Cursor::new(b""), u64::MAX).is_err());
    }

    #[test]
    fn rejects_short_and_excess_streams_with_bounded_reads() {
        let mut short = AuthenticatedInput::new(Cursor::new(b"abc"), 4).unwrap();
        short.read_to_end(&mut Vec::new()).unwrap();
        assert!(short.finish().is_err());
        let mut source = Cursor::new(b"0123456789");
        let mut excess = AuthenticatedInput::new(&mut source, 3).unwrap();
        assert_eq!(
            excess.read(&mut [0; 10]).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        assert!(excess.finish().is_err());
        assert_eq!(source.position(), 4);
    }

    #[test]
    fn a_decoder_cannot_swallow_a_failed_integrity_read() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(ErrorKind::InvalidData, "CRC mismatch"))
            }
        }
        let mut input = AuthenticatedInput::new(Broken, 0).unwrap();
        assert!(input.read(&mut [0; 1]).is_err());
        assert!(input.finish().is_err());
    }

    #[test]
    fn zip_crc_failure_is_not_hidden_by_exact_length_or_buffering() {
        use std::io::Write;
        use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
        let payload = b"authentic graph payload";
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file(
                "graph",
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        archive.write_all(payload).unwrap();
        let bytes = archive.finish().unwrap().into_inner();
        let mut original = ZipArchive::new(Cursor::new(bytes.clone())).unwrap();
        let offset = original.by_index_raw(0).unwrap().data_start() as usize;
        let mut corrupt = bytes;
        corrupt[offset] ^= 1;
        let mut archive = ZipArchive::new(Cursor::new(corrupt)).unwrap();
        let entry = archive.by_name("graph").unwrap();
        assert_eq!(entry.size(), payload.len() as u64);
        let mut input = AuthenticatedInput::new(entry, payload.len() as u64).unwrap();
        assert!(input.read_to_end(&mut Vec::new()).is_err());
        assert!(input.finish().is_err());
    }
}
