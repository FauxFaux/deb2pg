extern crate catfight;
extern crate tempdir;
extern crate tempfile;

use std::fs;
use std::io;

use io::Read;
use io::Seek;
use io::SeekFrom;
use io::Write;

#[test]
fn hello_world() {
    const BLOCK_SIZE: u64 = 1024;

    // file header length: implementation detail
    const FILE_HEADER_LEN: u64 = 16;

    let mut src = tempfile::tempfile().unwrap();
    src.write_all(b"hello").unwrap();
    src.seek(SeekFrom::Start(0)).unwrap();

    let dir = tempdir::TempDir::new("catfight-smoke").unwrap();

    assert_eq!(
        FILE_HEADER_LEN,
        catfight::store(
            BLOCK_SIZE,
            &mut src,
            dir.path().join("a").to_str().unwrap(),
            b"world",
        ).unwrap()
    );

    // implementation detail: file name
    let mut archive = fs::File::open(dir.path().join("a.0000000000000000000000")).unwrap();
    archive.seek(SeekFrom::Start(FILE_HEADER_LEN)).unwrap();
    {
        let mut record = catfight::read_record(&mut archive).unwrap().unwrap();
        let mut into = Vec::new();
        record.reader.read_to_end(&mut into).unwrap();

        assert_eq!(b"world", record.extra.as_slice());
        assert_eq!(b"hello", into.as_slice());
    }

    assert!(catfight::read_record(&mut archive).unwrap().is_none())
}
