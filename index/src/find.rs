use std;

use std::fs;
use std::io;
use std::path;
use std::slice;

use std::collections::HashSet;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

use lz4;
use twoway;

use catfight;
use memmap;
use names;
use tri;

const MAX_TRI: u32 = 64 * 64 * 64;

struct IndexFile<'f> {
    addendum: u64,
    map: memmap::Mmap,

    // this vec is really a box of an array[MAX_TRI], but "Stack Clash".
    by_tri: Vec<&'f [u32]>,
    pack: path::PathBuf,
}

pub struct Index<'i> {
    files: Vec<IndexFile<'i>>,
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


            let addendum = names::addendum_from_path(path.file_name().unwrap().to_str().unwrap());
            let map = memmap::Mmap::open_path(path, memmap::Protection::Read)?;

            assert_eq!(0, map.len() % std::mem::size_of::<u32>());
            let nums_len = map.len() / std::mem::size_of::<u32>();

            let raw = unsafe { slice::from_raw_parts(map.ptr() as *const u32, nums_len) };

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
        Ok(Index {
            files,
        })
    }

    pub fn documents_for_tri(&self, tri: u32) -> Vec<u64> {
        let mut all = Vec::new();
        for file in &self.files {
            all.extend(file.by_tri[tri as usize].iter().map(|x| *x as u64 + file.addendum));
        }
        all
    }

    pub fn documents_for_search(&self, search: &str) -> Vec<u64> {
        let mut matched = Vec::new();
        let target = tri::trigrams_full(search);
        for file in &self.files {
            let mut it = target.iter();
            let mut this_file: HashSet<u32> = match it.next() {
                Some(first) => file.by_tri[*first as usize].iter().map(|x| *x).collect(),
                None => continue,
            };

            // TODO: obviously this is dumb; they're pre-sorted
            while let Some(next) = it.next() {
                let next_set: HashSet<u32> = file.by_tri[*next as usize].iter().map(|x| *x).collect();
                this_file.retain(|x| next_set.contains(x));
                if this_file.is_empty() {
                    break;
                }
            }

            if this_file.is_empty() {
                continue;
            }

            let mut pack = fs::File::open(&file.pack).expect("pack shouldn't be deleted ever");
            for local in this_file {
                pack.seek(SeekFrom::Start(local as u64)).expect("seek");
                let mut entry = catfight::read_record(&mut pack).expect("read entry").expect("entry present");
                // TODO: This is quite dumb: could do early termination.

                // len is the compressed length, but better than zero
                let mut buf = Vec::with_capacity(entry.len as usize);
                lz4::Decoder::new(&mut entry.reader).unwrap().read_to_end(&mut buf).unwrap();
                if twoway::find_bytes(buf.as_slice(), search.as_bytes()).is_some() {
                    matched.push(local as u64 + file.addendum);
                }
            }

        }
        matched
    }
}
