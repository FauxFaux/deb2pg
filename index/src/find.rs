use std;

use std::io;
use std::path;
use std::slice;

use memmap;
use names;

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

impl<'i> Index<'i> {
    pub fn open(paths: &[&str]) -> io::Result<Self> {
        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            let addendum = names::addendum_from_path(path::Path::new(path).file_name().unwrap().to_str().unwrap());
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
