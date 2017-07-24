//! We're writing to 16 different files, and breaking those 16 different files at the 1gb marker.
//! Files may be bigger than 1gb, but an entry must start below 1gb.

use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;

use errors::*;
use names;

use catfight;

#[derive(Debug)]
struct Shard {
    file: File,
    nth: usize,
}

const MAGIC_MAX: u8 = 16;
const CHUNK_LEN_MAX: u64 = 1024 * 1024 * 1024;

#[derive(Debug)]
struct ShardedStore {
    shards: [Option<Shard>; MAGIC_MAX as usize],
}

impl ShardedStore {
    pub fn store(&mut self, src: &mut File, text: bool, extra: &[u8]) -> Result<u64> {
        let src_len: u64 = src.metadata()
            .chain_err(|| "couldn't stat source file")?
            .len();

        let magic = names::magic_offset_only(src_len, text);

        if self.shards[magic as usize].is_none() {
            self.shards[magic as usize] = Some(Shard {
                file: File::open(base_path + names::name_for_magic(magic))?,
                nth: 0,
            });
        }

        match fill_shard(self.shards[magic as usize].as_mut().unwrap(), src, src_len, extra) {
            Ok(pos) => Ok(magic as u64 + pos),
            Err(e) => {
                // if there was a problem, drop and close the file; fixes any locking concerns
                self.shards[magic as usize] = None;
                Err(e)
            }
        }
    }
}

fn fill_shard(shard: &mut Shard, src: &mut File, src_len: u64, extra: &[u8]) -> Result<u64> {
    loop {
        catfight::flock(&shard.file)?;

        let mut file_end: u64 = shard.file.seek(SeekFrom::End(0)).expect("seek on locked file");

        if file_end >= CHUNK_LEN_MAX {
            *shard = Shard {
                file: File::open(base_path + names::name_for_magic(magic))?,
                nth: shard.nth + 1,
            };
            continue;
        }

        catfight::writey_write(&mut shard.file, &mut file_end, src, src_len, extra)?;

        return Ok(shard.nth as u64 * CHUNK_LEN_MAX + file_end);
    }
}
