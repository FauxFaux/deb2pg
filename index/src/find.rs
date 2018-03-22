use std;

use std::fs;
use std::io;
use std::iter;
use std::path;
use std::slice;

use std::io::Seek;
use std::io::SeekFrom;

use lz4;

use catfight;
use grep;
use memmap;
use names;
use tri;

const MAX_TRI: u32 = 64 * 64 * 64;

#[derive(Debug)]
struct IndexFile<'f> {
    addendum: u64,
    map: memmap::Mmap,

    /// by_tri.len() === MAX_TRI. The contained slices are zero-or-more local document ids.
    by_tri: Vec<&'f [u32]>,
    pack: path::PathBuf,
}

pub struct Index<'i> {
    files: Vec<IndexFile<'i>>,
}

pub struct SearchResult {
    pub docs: Vec<u64>,
    pub grepped: u64,
}

impl<'i> Index<'i> {
    pub fn open(mut paths: Vec<path::PathBuf>) -> io::Result<Self> {
        paths.sort();
        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            let pack = {
                let mut tmp = path.clone();
                tmp.set_file_name(path.file_stem().expect("stem"));
                tmp
            };

            let (size_hint, addendum) =
                names::addendum_from_path(path.file_name().unwrap().to_str().unwrap());
            //            if size_hint > 4 {
            //                println!("limited size index: excluding files in {:?}", path);
            //                continue;
            //            }
            let file = fs::File::open(path)?;
            let map = unsafe { memmap::MmapOptions::new().map(&file)? };

            assert_eq!(0, map.len() % std::mem::size_of::<u32>());
            let nums_len = map.len() / std::mem::size_of::<u32>();

            let raw = unsafe { slice::from_raw_parts(map.as_ptr() as *const u32, nums_len) };

            let mut by_tri: Vec<&[u32]> = Vec::new();
            by_tri.resize(MAX_TRI as usize, &[]);

            let mut cur = 0;
            loop {
                // block header / guard
                let start = raw[cur];
                cur += 1;

                if 0 == start {
                    assert_eq!(cur, nums_len);
                    break;
                }

                assert_eq!(0xD81F, start, "{}", cur);

                assert_eq!(0, raw[cur]);
                cur += 1;
                assert_eq!(0, raw[cur]);
                cur += 1;

                // header length, in records
                let block_len = raw[cur];
                cur += 1;

                let mut block_cur = cur;

                cur += 2 * block_len as usize;

                // load all the headers,
                // cur is updated to skip over all the data
                for _ in 0..block_len {
                    let tri = raw[block_cur];
                    block_cur += 1;

                    let len = raw[block_cur];
                    block_cur += 1;

                    by_tri[tri as usize] = &raw[cur..(len as usize + cur)];

                    cur += len as usize;
                }
            }

            files.push(IndexFile {
                pack,
                addendum,
                map,
                by_tri,
            });
        }
        Ok(Index { files })
    }

    pub fn documents_for_tri(&self, tri: u32) -> Vec<u64> {
        let mut all = Vec::new();
        for file in &self.files {
            all.extend(
                file.by_tri[tri as usize]
                    .iter()
                    .map(|x| *x as u64 + file.addendum),
            );
        }
        all
    }

    pub fn documents_for_search(&self, search: &str) -> SearchResult {
        let mut matched = Vec::new();
        let mut grepped = 0u64;
        let target = tri::trigrams_full(search);
        for file in &self.files {
            let this_file = find_intersection(
                target
                    .iter()
                    .map(|tri| file.by_tri[*tri as usize].iter().peekable())
                    .collect(),
            );

            let mut pack = fs::File::open(&file.pack).expect("pack shouldn't be deleted ever");
            for local in this_file {
                pack.seek(SeekFrom::Start(local as u64)).expect("seek");
                let mut entry = catfight::read_record(&mut pack)
                    .expect("read entry")
                    .expect("entry present");

                if grep::reader_contains(
                    search.as_bytes(),
                    lz4::Decoder::new(&mut entry.reader).unwrap(),
                ).unwrap()
                    .is_some()
                {
                    matched.push(local as u64 + file.addendum);
                }
                grepped += 1;
            }
        }
        SearchResult {
            docs: matched,
            grepped,
        }
    }
}

fn find_intersection(mut slices: Vec<iter::Peekable<slice::Iter<u32>>>) -> Vec<u32> {
    slices.sort_unstable_by_key(|iter| iter.size_hint().0);

    let mut intersection: Vec<u32> = Vec::new();

    loop {
        // find the largest value at the head of a list; the next possible viable document
        let mut max = std::u32::MIN;
        for slice in &mut slices {
            match slice.peek() {
                Some(val) => if **val > max {
                    max = **val;
                },
                None => return intersection,
            }
        }

        // move everything up to this value, remembering whether a move was necessary
        let mut advanced = false;
        for slice in &mut slices {
            loop {
                let next = **match slice.peek() {
                    Some(new) => new,
                    None => return intersection,
                };
                if next >= max {
                    break;
                }

                advanced = true;
                slice.next();
            }
        }

        // if we didn't move anything up to the max, then we found a value
        if !advanced {
            intersection.push(max);
            for slice in &mut slices {
                if slice.next().is_none() {
                    return intersection;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::find_intersection;

    #[test]
    fn intersection() {
        let d1 = [1, 2, 3, 4, 6];
        let d2 = [2, 3, 4, 6];
        let d3 = [1, 2, 3, 4, 5];
        assert_eq!(
            vec![2, 3, 4],
            find_intersection(vec![
                d1.iter().peekable(),
                d2.iter().peekable(),
                d3.iter().peekable(),
            ])
        );
    }
}
