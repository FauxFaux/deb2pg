//! We're writing to 16 different files, and breaking those 16 different files at the 1gb marker.
//! Files may be bigger than 1gb, but an entry must start below 1gb.

use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::io;
use std::fs;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

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
pub struct ShardedStore {
    shards: [Option<Shard>; MAGIC_MAX as usize],
    base_path: PathBuf,
}

impl ShardedStore {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        ShardedStore {
            base_path: base_path.as_ref().to_path_buf(),
            shards: Default::default(),
        }
    }

    pub fn store(&mut self, src: &mut File, text: bool, extra: &[u8]) -> Result<u64> {
        let src_len: u64 = src.metadata()
            .chain_err(|| "couldn't stat source file")?
            .len();

        let magic = names::magic_offset_only(src_len, text);

        if self.shards[magic as usize].is_none() {
            self.shards[magic as usize] = Some(Shard {
                file: open_or_create_pack(&self.base_path, magic)?,
                nth: 0,
            });
        }

        match fill_shard(self.shards[magic as usize].as_mut().unwrap(), src, src_len, extra, self.base_path.as_path(), magic) {
            Ok(pos) => Ok(magic as u64 + pos),
            Err(e) => {
                // if there was a problem, drop and close the file; fixes any locking concerns
                self.shards[magic as usize] = None;
                Err(e)
            }
        }
    }
}

fn fill_shard(shard: &mut Shard, src: &mut File, src_len: u64, extra: &[u8], base_path: &Path, magic: u8) -> Result<u64> {
    loop {
        catfight::flock(&shard.file)?;

        let mut file_end: u64 = shard.file.seek(SeekFrom::End(0)).expect("seek on locked file");

        if file_end >= CHUNK_LEN_MAX {
            *shard = Shard {
                file: open_or_create_pack(base_path, magic)?,
                nth: shard.nth + 1,
            };
            continue;
        }

        catfight::writey_write(&mut shard.file, &mut file_end, src, src_len, extra)?;

        return Ok(shard.nth as u64 * CHUNK_LEN_MAX + file_end);
    }
}

fn open_or_create_pack<P: AsRef<Path>>(base_path: P, magic: u8) -> io::Result<File> {
    let mut new_path = base_path.as_ref().to_path_buf();
    new_path.push(names::name_for_magic(magic));
    return fs::OpenOptions::new().create(true).write(true).open(new_path);

    #[cfg(never)]
    match fs::OpenOptions::new().create_new(true).write(true).open(new_path) {
        Ok(mut file) => {
            file.write(b"\0...")?;
            Ok(file)
        },
        Err(ref e) if io::ErrorKind::AlreadyExists == e.kind() => fs::File::open(new_path),
        Err(ref e) => Err(e),
    }
}
