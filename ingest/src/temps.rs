use std::io::Read;
use std::io::Write;
use std::path::Path;

use tempfile_fast::PersistableTempFile;
use tempfile_fast::persistable_tempfile_in;
use sha2::Digest;
use sha2::Sha512Trunc256;
use zstd;

use errors::*;

pub struct TempFile {
    pub len: u64,
    pub hash: [u8; 256 / 8],
    pub file: PersistableTempFile,
}

fn is_text(buf: &[u8]) -> bool {
    for char in buf {
        if 0 == *char // NUL
            // ENQ (enquiry), ACK (acknowledge),
            // \a (bell) and \b (backspace)
            || (*char >= 5 && *char <= 8)
            // SO, SI, DLE, DC?, NAK, SYN, ETB, CAN, EM, SUB, ESC (colour codes?),
            // FS, GS, RS, US
            || (*char >= 14 && *char < 32)
        {
            return false;
        }
    }

    true
}

pub fn hash_compress_write_from_reader<R: Read, P: AsRef<Path>>(
    mut from: R,
    inside: P,
) -> Result<Option<TempFile>> {
    let mut to = persistable_tempfile_in(inside)?;
    let mut hasher = Sha512Trunc256::default();
    let mut total_read = 0u64;

    {
        let mut compressor = zstd::Encoder::new(to.as_mut(), 0)?;

        loop {
            let mut buf = [0u8; 1024 * 8];

            let read = from.read(&mut buf)?;
            if 0 == read {
                break;
            }

            if !is_text(&buf[0..read]) {
                return Ok(None);
            }

            total_read += read as u64;

            hasher.input(&buf[0..read]);
            compressor.write_all(&buf[0..read])?;
        }
        compressor.finish()?;
    }

    Ok(Some(TempFile {
        len: total_read,
        hash: to_bytes(&hasher.result()[..]),
        file: to,
    }))
}

fn to_bytes(slice: &[u8]) -> [u8; 256 / 8] {
    let mut hash = [0u8; 256 / 8];
    hash.clone_from_slice(slice);
    hash
}
