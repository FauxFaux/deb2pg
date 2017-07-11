extern crate byteorder;
extern crate catfight;
extern crate index;
extern crate lz4;
extern crate memmap;
extern crate tempfile;

use std::env;
use std::fs;
use std::io;

use std::collections::HashMap;

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
    let mut out = io::BufWriter::new(fs::File::create(&args[2]).unwrap());

    let mut pos = 16;
    fp.seek(SeekFrom::Start(pos)).unwrap();

    let mut idx = Vec::with_capacity(200_000);
    let mut temp = io::BufWriter::new(tempfile::tempfile().unwrap());
    let mut seen: HashMap<u32, u32> = HashMap::with_capacity(64*64);
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
                *seen.entry(t).or_insert(0) += 1;

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

    println!("{} seen", seen.len());

    // break seen into blocks of < 1,000,000 positions
    // scan the file for those seens; by summing all the things in the range of subslice
    // write those out?

    let mut seen: Vec<(u32, u32)> = seen.iter().map(|x| (*x.0, *x.1)).collect();

    seen.sort();

    let mut seen = seen.iter();

    loop {
        let mut block = 0usize;
        let mut tris: HashMap<u32, Vec<u32>> = HashMap::new();
        loop {
            let (tri, count) = match seen.next() {
                Some(t) => (t.0, t.1),
                None => break,
            };

            tris.insert(tri, Vec::with_capacity(count as usize));

            // ~ 400MB ram usage?
            if block > 100_000_000 {
                break;
            }
            block += count as usize;
        }

        if tris.is_empty() {
            break;
        }

        // inclusive
        let min = *tris.keys().min().unwrap();
        let max = *tris.keys().max().unwrap();

        let mut run = 0usize;

        for part in &idx {
            let subslice = &whole[run..(run + (part.len as usize))];
            let mut start = match subslice.binary_search(&min) {
                Ok(idx) | Err(idx) => idx,
            };

            if start > 1 {
                start -= 2;
            }

            let end = start + match subslice[start..].binary_search(&max) {
                Ok(idx) | Err(idx) => idx,
            } + 1;

            let end = if end > subslice.len() {
                subslice.len()
            } else {
                end
            };

            for tri in &subslice[start..end] {
                if let Some(v) = tris.get_mut(tri) {
                    v.push(part.pos);
                }
            }

            run += part.len as usize;
        }

        assert_eq!(map.len(), run * 4);

        for (tri, poses) in &tris {
            assert_eq!(tris[tri].len(), poses.len(), "{}", tri);
            out.write_u32::<LittleEndian>(*tri).unwrap();

            assert!(poses.len() <= std::u32::MAX as usize);
            out.write_u32::<LittleEndian>(poses.len() as u32).unwrap();

            for pos in poses {
                out.write_u32::<LittleEndian>(*pos).unwrap();
            }
        }
    }

    // zeroth's trigram has zero items
    out.write_u32::<LittleEndian>(0).unwrap();
    out.write_u32::<LittleEndian>(0).unwrap();
}
