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
fn convert_pack_to_just_trigrams<R: Read + Seek>(mut pack: R) -> (fs::File, Vec<TempFileChunk>, HashMap<Tri, Count>) {
    let mut pos = 16;
    pack.seek(SeekFrom::Start(pos)).unwrap();

    let mut temp = io::BufWriter::new(tempfile::tempfile().unwrap());
    let mut temp_index = Vec::with_capacity(200_000);
    let mut trigram_count: HashMap<Tri, u32> = HashMap::with_capacity(64*64*64);

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

/// Take the lowest trigrams out of the iterator, and prepare space to gather Poses for them.
/// Stops at an arbitrary memory limit.
fn take_some<I>(trigram_count: &mut I) -> HashMap<Tri, Vec<Pos>>
    where
        I: Iterator<Item=(Tri, Count)>
{
    let mut block = 0usize;
    let mut tris: HashMap<Tri, Vec<Pos>> = HashMap::with_capacity(128);
    loop {
        let (tri, count) = match trigram_count.next() {
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

    tris.shrink_to_fit();
    tris
}

fn fill_tris(temp_index: &[TempFileChunk], temp_data: &[Tri], tris: &mut HashMap<Tri, Vec<Pos>>) {
    // inclusive
    let min = *tris.keys().min().unwrap();
    let max = *tris.keys().max().unwrap();

    let mut run = 0usize;

    // For every document...
    for part in temp_index {

        // Work out what part of its trigrams we even need to look at...
        let subslice = &temp_data[run..(run + (part.num_tris as usize))];
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

        // Then, for each of those, save the document ids in the appropriate place.
        for tri in &subslice[start..end] {
            if let Some(v) = tris.get_mut(tri) {
                v.push(part.pos);
            }
        }

        run += part.num_tris as usize;
    }

    assert_eq!(temp_data.len(), run);
}


fn main() {
    let args: Vec<String> = env::args().collect();

    let fp = io::BufReader::new(fs::File::open(&args[1]).unwrap());
    let mut out = io::BufWriter::new(fs::File::create(&args[2]).unwrap());

    // First, we transform the pack into just the trigrams for each item in the pack,
    // remembering where those trigrams referred to, stored in a `temp`orary file.
    let (temp, temp_index, trigram_count) = convert_pack_to_just_trigrams(fp);


    // Sort the trigrams we've seen by number.
    let mut trigram_count: Vec<(Tri, Count)> = trigram_count.into_iter().map(|x| (x.0, x.1)).collect();
    trigram_count.sort();
    let mut trigram_count = trigram_count.into_iter();


    // ..which we immediately map into memory as an array of Tri(u32)s.
    let map = memmap::Mmap::open(&temp, memmap::Protection::Read).unwrap();
    let temp_data = unsafe { std::slice::from_raw_parts((map.ptr()) as *const Tri, map.len() / 4) };


    // Now, we want to transpose [{A, B, C}, {A, C, E}] into [A: {1, 2}, B: {1}, C: {1, 2}, E: {2}]


    // Let's do this in blocks, so we don't run out of memory.
    loop {

        // Select a block of trigrams to process, and allocate space for their documents.
        let mut tri_poses = take_some(&mut trigram_count);

        if tri_poses.is_empty() {
            break;
        }

        // Fetch the document ids for each of the selected trigrams, by scanning the temp data.
        fill_tris(&temp_index, &temp_data, &mut tri_poses);

        let mut tris: Vec<Tri> = tri_poses.keys().map(|x| *x).collect();
        tris.sort();

        // Write them out.

        // Format (everything is a u32):
        // file: [block] [block..] 0
        // block: [num headers] [header] [header..] [poses] [poses..]
        // header: [tri] [num tris]
        // poses: ..just the raw data

        // num headers
        out.write_u32::<LittleEndian>(tri_poses.len() as u32).unwrap();

        for tri in &tris {
            out.write_u32::<LittleEndian>(*tri).unwrap();

            let poses = &tri_poses[tri];
            assert!(poses.len() <= std::u32::MAX as usize);
            out.write_u32::<LittleEndian>(poses.len() as u32).unwrap();
        }

        for tri in tris {
            let poses = &tri_poses[&tri];

            assert!(poses.len() <= std::u32::MAX as usize);
            out.write_u32::<LittleEndian>(poses.len() as u32).unwrap();

            for pos in poses {
                out.write_u32::<LittleEndian>(*pos).unwrap();
            }
        }
    }

    // zero item header -> end of file
    out.write_u32::<LittleEndian>(0).unwrap();
}
