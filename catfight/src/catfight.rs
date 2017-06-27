use std;
use std::io;
use std::fs;
use std::fs::File;

use libc;

use copy::copy_file;

use errors::*;

use std::io::Seek;
use std::io::Write;
use std::os::unix::io::AsRawFd;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

pub fn align(val: u64) -> u64 {
    (val + 15) / 16 * 16
}

fn unarchive(root: &str, block_size: u64, offset: u64) -> Result<()> {
    let target_file_id: u64 = offset / block_size;
    let target_file_offset = offset % block_size;

    let target_path = format!("{}.{:022}", root, target_file_id);
    let mut fd = File::open(target_path)?;
    let file_len = fd.seek(io::SeekFrom::End(0))?;

    fd.seek(io::SeekFrom::Start(target_file_offset))?;
    let end = fd.read_u64::<LittleEndian>()?;
    let extra_len = fd.read_u64::<LittleEndian>()?;

    ensure!(end >= 8 + 8, "there isn't even a header, invalid offset?");

    ensure!(
        extra_len < std::i64::MAX as u64,
        "extra length is far too large, invalid offset?"
    );

    ensure!(
        extra_len <= end - 8 - 8,
        "too much extra data for record; invalid offset?"
    );

    ensure!(
        target_file_offset + end <= file_len,
        "record extends beyond end of file; invalid offset?"
    );

    fd.seek(io::SeekFrom::Current(extra_len as i64))?;

    copy_file(&mut fd, &mut io::stdout(), end - extra_len - 8 - 8)?;

    Ok(())
}

fn flock(what: &File) -> Result<()> {
    let ret = unsafe { libc::flock(what.as_raw_fd(), libc::LOCK_EX) };
    if 0 != ret {
        Err(Error::with_chain(io::Error::last_os_error(), "flocking"))
    } else {
        Ok(())
    }
}

fn unlock_flock(what: &File) -> Result<()> {
    let ret = unsafe { libc::flock(what.as_raw_fd(), libc::LOCK_UN) };
    if 0 != ret {
        Err(Error::with_chain(io::Error::last_os_error(), "un-flocking"))
    } else {
        Ok(())
    }
}

pub fn store(blocksize: u64, src_path: &str, dest_root: &str, extra: &str) -> Result<u64> {
    let mut src = File::open(src_path).chain_err(
        || "couldn't open source file",
    )?;

    let src_len: u64 = fs::metadata(src_path)
        .chain_err(|| "couldn't stat source file")?
        .len();

    for target_num in 0..std::u64::MAX {
        let target_path = format!("{}.{:022}", dest_root, target_num);
        let mut fd = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(target_path.as_str())
            .unwrap();

        flock(&fd)?;

        let mut file_end: u64 = fd.seek(io::SeekFrom::End(0))?;

        if file_end >= blocksize {
            continue;
        }

        ensure!(
            0 == file_end % 16,
            ErrorKind::InvalidState(format!("unaligned file: {}", file_end))
        );

        if 0 == file_end {
            // we locked a new file, write a header
            fd.write_all(b"cf2\0\0\0\0\0")?;
            fd.write_u64::<LittleEndian>(target_num).unwrap();
            file_end = 16;
        }

        let extra_len: u64 = extra.len() as u64;
        let record_end = 8 + 8 + src_len + extra_len;
        fd.write_u64::<LittleEndian>(record_end)?;
        fd.write_u64::<LittleEndian>(extra_len)?;
        fd.write_all(extra.as_bytes())?;
        fd.flush()?;

        fd.set_len(file_end + align(record_end))?;

        unlock_flock(&fd)?;

        copy_file(&mut src, &mut fd, src_len)?;

        return Ok(file_end);
    }

    Err(
        ErrorKind::InvalidState("ran out of files".to_string()).into(),
    )
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
