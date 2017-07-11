use std;

use std::fs;
use std::io;
use std::path;
use std::slice;

use memmap;

const MAX_TRI: u32 = 64 * 64 * 64;

struct IndexFile<'f> {
    addendum: u64,
    map: memmap::Mmap,

    // this vec is really a box of an array[MAX_TRI], but "Stack Clash".
    by_tri: Vec<&'f [u32]>,
}

pub struct Index<'i> {
    files: Vec<IndexFile<'i>>,
}

// TODO: this is awful
fn addendum_from_path(path: &str) -> u64 {
    // text-5.0000000000000000000000.idx
    assert!(path.starts_with("text-"), "path must start with 'text-', not {}", path);
    let mut it = path.chars().skip("text-".len());
    let size_raw = it.next().expect("num") as u8 - '2' as u8;
    assert!(size_raw >= 2 && size_raw <= 9);
    let size_raw = size_raw as u64;
    assert_eq!('.', it.next().unwrap());

    // TODO: NO JUST NO WHY
    it.take("0000000000000000000000".len()).collect::<String>().parse::<u64>().expect("second num")
        * 1024 * 1024 * 1024 + size_raw - 2 + 8
}

impl<'i> Index<'i> {
    pub fn open(paths: &[&str]) -> io::Result<Self> {
        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            let addendum = addendum_from_path(path::Path::new(path).file_name().unwrap().to_str().unwrap());
            let map = memmap::Mmap::open_path(path, memmap::Protection::Read)?;

            assert_eq!(0, map.len() % std::mem::size_of::<u32>());
            let nums_len = map.len() / std::mem::size_of::<u32>();

            let raw = unsafe { slice::from_raw_parts(map.ptr() as *const u32, nums_len) };

            let mut by_tri: Vec<&[u32]> = Vec::new();
            by_tri.resize(MAX_TRI as usize, &[]);

            let mut cur = 0;
            loop {
                if cur == nums_len {
                    break;
                }

                let tri = raw[cur];
                cur += 1;

                let len = raw[cur];
                cur += 1;

                by_tri[tri as usize] = &raw[cur..(len as usize + cur)];

                cur += len as usize;
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
