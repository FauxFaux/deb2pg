use std;

use std::fs;
use std::io;
use std::slice;

use memmap;

const MAX_TRI: u32 = 64 * 64 * 64;

struct IndexFile<'f> {
    addendum: u64,
    map: memmap::Mmap,
    by_tri: [&'f [u32]; MAX_TRI as usize],
}

pub struct Index<'i> {
    files: Vec<IndexFile<'i>>,
}

fn addendum_from_path(path: &str) -> u64 {
    assert!(path.starts_with("text-"), "path must start with 'text-', not {}", path);
    let mut it = path.chars().skip("text-".len());
    let size_raw = it.next().expect("num") as u8 - '2' as u8;
    assert!(size_raw >= 2 && size_raw <= 9);
    let size_raw = size_raw as u64;
    assert_eq!('.', it.next().unwrap());
    it.collect::<String>().parse::<u64>().expect("second num")
        * 1024 * 1024 * 1024 + size_raw - 2
}

impl<'i> Index<'i> {
    pub fn open(paths: &[&str]) -> io::Result<Self> {
        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            let addendum = addendum_from_path(path);
            let map = memmap::Mmap::open_path(path, memmap::Protection::Read)?;

            let raw = {
                let nums_ptr = map.ptr() as *const u32;

                assert_eq!(0, map.len() % std::mem::size_of::<u32>());
                let nums_len = map.len() / std::mem::size_of::<u32>();

                unsafe { slice::from_raw_parts(nums_ptr, nums_len) }
            };

            let mut by_tri: [&[u32]; MAX_TRI as usize] = [&[]; MAX_TRI as usize];

            let mut cur = 0;
            loop {
                let tri = raw[cur];
                cur += 1;

                let len = raw[cur];
                cur += 1;

                by_tri[tri as usize] = &raw[cur..(len as usize + cur)];
            }

            files.push(IndexFile {
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
}
