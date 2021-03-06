use std;
use std::io;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::os::unix::io::AsRawFd;

use libc;
use copy::copy_file;
use errors::*;
use iowrap::Eof;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

pub fn align(val: u64) -> u64 {
    (val + 15) / 16 * 16
}

pub struct Record<R>
where
    R: io::Read,
{
    pub reader: io::Take<Eof<R>>,
    pub extra: Vec<u8>,
    pub len: u64,
    realign: u8,
}

impl<'a, R> Record<R>
where
    R: std::io::Read,
{
    pub fn len(&self) -> u64 {
        self.extra.len() as u64 + 16 + self.len as u64 + self.realign as u64
    }

    /// Assuming the reader has been consumed, realign us with where the next record should be.
    pub fn complete(mut self) -> Result<()> {
        let mut buf = vec![0u8; self.realign as usize];
        let mut byte = [0u8, 1];
        if 0 != self.reader.read(&mut byte)? {
            bail!(ErrorKind::InvalidState(
                "can't complete until the 'reader' has been fully read".to_string(),
            ));
        }

        self.reader.into_inner().read_exact(&mut buf)?;
        Ok(())
    }
}

pub fn read_record<R: io::Read>(fd: R) -> Result<Option<Record<R>>> {
    let mut fd = Eof::new(fd);
    if fd.eof()? {
        return Ok(None);
    }

    let end = fd.read_u64::<LittleEndian>()?;
    let extra_len = fd.read_u64::<LittleEndian>()?;

    ensure!(end >= 8 + 8, "there isn't even a header, invalid offset?");
    ensure!(
        extra_len < std::i64::MAX as u64 && extra_len < std::usize::MAX as u64,
        "extra length is far too large, invalid offset?"
    );
    ensure!(
        extra_len <= end - 8 - 8,
        "too much extra data for record; invalid offset?"
    );

    let mut extra = vec![0u8; extra_len as usize];
    fd.read_exact(&mut extra)?;

    let len = end - 8 - 8 - extra_len;

    Ok(Some(Record {
        len,
        reader: fd.take(len),
        extra,
        realign: (align(end) - end) as u8,
    }))
}

pub fn flock(what: &File) -> Result<()> {
    let ret = unsafe { libc::flock(what.as_raw_fd(), libc::LOCK_EX) };
    if 0 != ret {
        Err(Error::with_chain(io::Error::last_os_error(), "flocking"))
    } else {
        Ok(())
    }
}

pub fn unlock_flock(what: &File) -> Result<()> {
    let ret = unsafe { libc::flock(what.as_raw_fd(), libc::LOCK_UN) };
    if 0 != ret {
        Err(Error::with_chain(io::Error::last_os_error(), "un-flocking"))
    } else {
        Ok(())
    }
}

pub fn writey_write(
    fd: &mut File,
    file_end: &mut u64,
    src: &mut File,
    src_len: u64,
    extra: &[u8],
) -> Result<()> {
    ensure!(
        0 == *file_end % 16,
        ErrorKind::InvalidState(format!("unaligned file: {}", file_end))
    );

    if 0 == *file_end {
        // we locked a new file, write a header
        fd.write_all(b"cf2\0\0\0\0\0\0\0\0\0\0\0\0\0")?;
        *file_end = 16;
    }

    let extra_len: u64 = extra.len() as u64;
    let record_end = 8 + 8 + src_len + extra_len;
    fd.write_u64::<LittleEndian>(record_end)?;
    fd.write_u64::<LittleEndian>(extra_len)?;
    fd.write_all(extra)?;
    fd.flush()?;

    fd.set_len(*file_end + align(record_end))?;

    unlock_flock(&fd)?;

    copy_file(src, fd, src_len)?;
    Ok(())
}

#[cfg(never)]
fn find_by_listing() {
    for candidate in fs::read_dir(dest_root)? {
        let candidate = candidate?;
        let name = candidate.file_name();
        if name.len() < 22 || '.' == name[0] || name[name.len() - 4..] != ".ppg" {
            continue;
        }

        if candidate.metadata()?.len() > blocksize {
            continue;
        }

        let file = fs::OpenOptions::new().write(true).open(candidate.path())?;
        flock(file)?;

        unlock_flock(file)?
    }
}
