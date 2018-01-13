use std::fs;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use fs2::FileExt;
use tempfile_fast::PersistableTempFile;

use errors::*;

pub struct ShardedStore {
    outdir: PathBuf,
    loose: u64,
}

// https://docs.google.com/spreadsheets/d/14LIFzEZt_I0MJeH-gOkMd3iGzkO_2YP4yGF0zoIP19U/edit?usp=sharing
// How about: 32 byte segments, 256 packs, so covering up to 8kb, then loose files from then on.
// 8kb covers 95% of non-binary files in my test packages.
// Saving some of that 256 might help with indexes, but doesn't help much with number of documents:
//   2^(64 - lg2(256)) = 2^56 = 72 million billion documents.
// Smaller segments? Don't like the idea of going below 8 bytes? 8kb coverage -> 1024 packs.
//  2^(64 - lg2(1024)) = 2^54 = still millions of billions.
// 2^40 (a thousand billion) still sounds like a lot. 2^(64-40) = 16M. We could do byte aligned all
// the way up to there? Insanity.
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
        let mut pack = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(pack_path)?;
        pack.lock_exclusive()?;
        let loc = pack.seek(SeekFrom::End(0))?;
        pack.write_all(&buf)?;
        Ok(loc)
    }
}
