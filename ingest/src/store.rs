use std::fs;
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
// The original reason for a small number of packs was to enable them to all remain open on a system
// with a small number of allowed open files. Ubuntu seems to default to 1024. This isn't a real
// restriction, though, as clearly this is a server application now.

const SEGMENT_SIZE: u64 = 16;
const SEGMENTS: u64 = 512;
// 512 * 16 = 8k.


impl ShardedStore {
    pub fn new<P: AsRef<Path>>(outdir: P) -> ShardedStore {
        ShardedStore {
            outdir: outdir.as_ref().to_path_buf(),
        }
    }

    pub fn store<F>(&mut self, mut file: PersistableTempFile, next_loose: F) -> Result<u64>
    where
        F: FnOnce() -> Result<u64>,
    {
        let len = file.seek(SeekFrom::End(0))?;
        assert_ne!(0, len);
        let id = ((len - 1) / SEGMENT_SIZE) + 1;
        // segments: 4, 0: loose, 1, 2, 3: packed. 3 < 4.
        Ok(if id < SEGMENTS {
            self.store_pack(id, len, &mut file)? * SEGMENTS + id
        } else {
            let loose = next_loose()?;
            let first = loose % 0x100;
            let second = (loose / 0x100) % 0x100;
            let mut ret = self.outdir.to_path_buf();
            ret.push(format!("{:02x}", first));
            ret.push(format!("{:02x}", second));
            ret.push(format!("{:x}.loose", loose));
            file.persist_noclobber(ret)?;
            loose * SEGMENTS
        })
    }

    pub fn locality(&self) -> &Path {
        self.outdir.as_path()
    }


    fn store_pack(&self, id: u64, len: u64, file: &mut PersistableTempFile) -> Result<u64> {
        assert_lt!(id, SEGMENTS);
        file.seek(SeekFrom::Start(0))?;
        let eventual_size = (id * SEGMENT_SIZE) as usize;
        let mut buf = Vec::with_capacity(eventual_size);
        file.read_to_end(&mut buf)?;
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
