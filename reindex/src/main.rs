extern crate byteorder;
extern crate catfight;
extern crate index;
extern crate lz4;
extern crate memmap;
extern crate tempfile;

use std::env;
use std::fs;
use std::io;

use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

use byteorder::{LittleEndian, WriteBytesExt};

struct Idx {
    pos: u32,
    len: u32,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut fp = io::BufReader::new(fs::File::open(&args[1]).unwrap());
    fp.seek(SeekFrom::Start(16)).unwrap();

    let mut idx = Vec::with_capacity(200_000);
    let mut temp = io::BufWriter::new(tempfile::tempfile().unwrap());
    loop {
        let pos = fp.seek(SeekFrom::Current(0)).unwrap();
        if let Some(mut entry) = catfight::read_record(&mut fp).unwrap() {
            // len is the compressed length, but better than zero
            let mut buf = Vec::with_capacity(entry.len as usize);
            lz4::Decoder::new(&mut entry.reader).unwrap().read_to_end(&mut buf).unwrap();

            let tris = index::trigrams_full(&String::from_utf8_lossy(&buf));
            idx.push(Idx {
                pos: pos as u32,
                len: tris.len() as u32,
            });

            for t in tris {
                // TODO: this actually should be in platform endian
                temp.write_u32::<LittleEndian>(t).unwrap();
            }

            entry.complete().unwrap();
        } else {
            break;
        }

        // Basically random, although does need to be mod 16.
        if 0 == (pos % (16 * 1024)) {
            println!("{:0.2}", pos as f32 / 1e9 * 100.0f32);
        }
    }

    let mut temp = temp.into_inner().unwrap();

    temp.flush().unwrap();

    let map = memmap::Mmap::open(&temp, memmap::Protection::Read).unwrap();

    let whole = unsafe { std::slice::from_raw_parts((map.ptr()) as *const u32, map.len() / 4) };

    let mut out = io::BufWriter::new(fs::File::create("o").unwrap());

    for tri in 0..(64u32*64*64) {
        let mut run = 0usize;
        let mut poses = Vec::with_capacity(100);
        for part in &idx {
            if whole[run..(run+(part.len as usize))].binary_search(&tri).is_ok() {
                poses.push(part.pos);
            }

            run += part.len as usize;
        }

        println!("{}: {}", tri, poses.len());

        out.write_u64::<LittleEndian>(poses.len() as u64).unwrap();
        for pos in poses {
            out.write_u32::<LittleEndian>(pos).unwrap();
        }
    }
}
