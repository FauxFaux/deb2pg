extern crate byteorder;
extern crate catfight;
extern crate index;
extern crate lz4;
extern crate memmap;
extern crate tempfile;

use std::env;
use std::fs;
use std::io;

use std::collections::HashSet;

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
    let mut pos = 16;

    let mut idx = Vec::with_capacity(200_000);
    let mut temp = io::BufWriter::new(tempfile::tempfile().unwrap());
    let mut seen = HashSet::with_capacity(64*64);
    loop {
        if let Some(mut entry) = catfight::read_record(&mut fp).unwrap() {
            // len is the compressed length, but better than zero
            let mut buf = Vec::with_capacity(entry.len as usize);
            lz4::Decoder::new(&mut entry.reader).unwrap().read_to_end(&mut buf).unwrap();

            let mut tris = index::trigrams_full(&String::from_utf8_lossy(&buf));
            idx.push(Idx {
                pos: pos as u32,
                len: tris.len() as u32,
            });

            tris.sort();

            for t in tris {
                seen.insert(t);
                // TODO: this actually should be in platform endian
                temp.write_u32::<LittleEndian>(t).unwrap();
            }

            pos += entry.len();

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

    println!("{} seen", seen.len());

    for tri in seen {
        let mut run = 0usize;
        let mut poses = Vec::with_capacity(100);
        for part in &idx {
            let subslice = &whole[run..(run + (part.len as usize))];
            if subslice.binary_search(&tri).is_ok() {
                poses.push(part.pos);
            }

            run += part.len as usize;
        }

        assert_eq!(map.len(), run * 4);
        assert_ne!(0, poses.len());

        out.write_u64::<LittleEndian>(poses.len() as u64).unwrap();
        for pos in poses {
            out.write_u32::<LittleEndian>(pos).unwrap();
        }
    }
}
