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

type Pos = u32;
type Tri = u32;
type Count = u32;

struct TempFileChunk {
    num_tris: Count,
    pos: Pos,
}

/// Consume a pack file, and generate a temporary file containing the trigrams for every entry.
/// `temp` file contains no metadata, literally just a concatenation of the tris for the first pos,
/// then for the second, ...
/// The returned `temp_index` of chunks stores their length, and which chunk-relative 'pos' they refer to
/// The `trigram_count` for every trigram is also recorded.
fn read_pack<R: Read + Seek>(mut pack: R) -> (fs::File, Vec<TempFileChunk>, HashMap<Tri, Count>) {
    let mut pos = 16;
    pack.seek(SeekFrom::Start(pos)).unwrap();

    let mut temp = io::BufWriter::new(tempfile::tempfile().unwrap());
    let mut temp_index = Vec::with_capacity(200_000);
    let mut trigram_count: HashMap<Tri, u32> = HashMap::with_capacity(64*64);

    loop {
        if let Some(mut entry) = catfight::read_record(&mut pack).unwrap() {
            // len is the compressed length, but better than zero
            let mut buf = Vec::with_capacity(entry.len as usize);
            lz4::Decoder::new(&mut entry.reader).unwrap().read_to_end(&mut buf).unwrap();

            let mut tris: Vec<u32> = index::trigrams_full(&String::from_utf8_lossy(&buf)).into_iter().collect();
            temp_index.push(TempFileChunk {
                pos: pos as u32,
                num_tris: tris.len() as u32,
            });

            tris.sort();

            for t in tris {
                *trigram_count.entry(t).or_insert(0) += 1;

                // TODO: this actually should be in platform endian
                temp.write_u32::<LittleEndian>(t).unwrap();
            }

            pos += entry.len();

            entry.complete().unwrap();
        } else {
            break;
        }
    }

    let mut temp = temp.into_inner().unwrap();

    temp.flush().unwrap();

    (temp, temp_index, trigram_count)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let fp = io::BufReader::new(fs::File::open(&args[1]).unwrap());
    let mut out = io::BufWriter::new(fs::File::create(&args[2]).unwrap());

    let (temp, temp_index, trigram_count) = read_pack(fp);

    let map = memmap::Mmap::open(&temp, memmap::Protection::Read).unwrap();

    let whole = unsafe { std::slice::from_raw_parts((map.ptr()) as *const u32, map.len() / 4) };

    println!("{} seen", trigram_count.len());

    let mut trigram_count: Vec<(u32, u32)> = trigram_count.into_iter().map(|x| (x.0, x.1)).collect();

    trigram_count.sort();

    let mut seen = trigram_count.iter();

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

        for part in &temp_index {
            let subslice = &whole[run..(run + (part.num_tris as usize))];
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

            run += part.num_tris as usize;
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
