use std::fs;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

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
const SEGMENTS: u64 = 4096;
//  512 * 16 = 8k.
// 1024 * 32 = 32k.
// 4096 * 16 = 64k.

const MAGIC_LEN: u64 = 4;

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
        let len = file.seek(SeekFrom::End(0))? - MAGIC_LEN;
        assert_gt!(len, 2);
        let id = ((len - 1) / SEGMENT_SIZE) + 1;
        // segments: 4, 0: loose, 1, 2, 3: packed. 3 < 4.
        Ok(if id < SEGMENTS {
            self.store_pack(id, len, &mut file)? * SEGMENTS + id
        } else {
            let loose = next_loose()?;
            let first = loose % 0x1000;
            let mut ret = self.outdir.to_path_buf();
            ret.push("loose");
            ret.push(format!("{:03x}", first));
            ret.push(format!("{:x}", loose));
            file.persist_noclobber(ret)
                .chain_err(|| "writing loose object")?;
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
        assert_eq!(len + MAGIC_LEN, buf.len() as u64);
        buf.extend(vec![0; eventual_size - len as usize]);
        let mut pack_path = self.outdir.to_path_buf();
        pack_path.push("packs");
        pack_path.push(format!("{:x}", id % 0x10));
        pack_path.push(format!("{:03x}.pack", id));
        let mut pack = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&pack_path)?;
        ensure!(
            pack.seek(SeekFrom::End(0))? % (eventual_size as u64) == 0,
            "file is improper: {:?}",
            pack_path
        );
        let to_write = &buf[MAGIC_LEN as usize..];
        ensure!(
            to_write.len() == pack.write(to_write)?,
            "atomic write failed, uh oh"
        );
        Ok(pack.seek(SeekFrom::Current(0))?)
    }
}
