use std::fs;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::path::PathBuf;


use tempfile_fast::PersistableTempFile;
use libc;

use errors::*;

pub struct ShardedStore {
    outdir: PathBuf,
    loose: u64,
}

impl ShardedStore {
    pub fn new<P: AsRef<Path>>(outdir: P) -> ShardedStore {
        ShardedStore {
            outdir: outdir.as_ref().to_path_buf(),
            loose: 0,
        }
    }

    pub fn store(&mut self, mut file: PersistableTempFile, hash: &[u8]) -> Result<i64> {
        let len = file.seek(SeekFrom::End(0))?;
        Ok(if len < 64 * (255 - 1) {
            let id = len / 64;
            self.store_pack(id, len, &mut file)? * 0x100 + id
        } else {
            loop {
                self.loose += 1;
                match file.persist_noclobber(self.loose_path()) {
                    Ok(()) => break,
                    Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(e) => bail!(e),
                }
            }
            self.loose
        } as i64)
    }

    pub fn locality(&self) -> &Path {
        self.outdir.as_path()
    }

    fn loose_path(&self) -> PathBuf {
        let first = self.loose % 0x100;
        let second = (self.loose / 0x100) % 0x100;
        let mut ret = self.outdir.to_path_buf();
        ret.push(format!("{:02x}", first));
        ret.push(format!("{:02x}", second));
        ret.push(format!("{:x}.loose", self.loose));
        ret
    }

    fn store_pack(&self, id: u64, len: u64, file: &mut PersistableTempFile) -> Result<u64> {
        file.seek(SeekFrom::Start(0))?;
        let eventual_size = ((id + 1) * 64) as usize;
        let mut buf = Vec::with_capacity(eventual_size);
        file.read_to_end(&mut buf);
        assert_eq!(len, buf.len() as u64);
        buf.extend(vec![0; eventual_size - len as usize]);
        let mut pack_path = self.outdir.to_path_buf();
        pack_path.push(format!("{:02x}.pack", id));
        let mut pack = fs::OpenOptions::new().create(true).append(true).open(pack_path)?;
        flock(&pack)?;
        let loc = pack.seek(SeekFrom::End(0))?;
        pack.write_all(&buf)?;
        Ok(loc)
    }
}


pub fn flock(what: &fs::File) -> Result<()> {
    let ret = unsafe { libc::flock(what.as_raw_fd(), libc::LOCK_EX) };
    if 0 != ret {
        Err(Error::with_chain(io::Error::last_os_error(), "flocking"))
    } else {
        Ok(())
    }
}