extern crate byteorder;
extern crate index;

use std::env;
use std::fs;
use std::io;

use io::Read;
use io::Seek;
use io::SeekFrom;

use byteorder::{LittleEndian, ReadBytesExt};

fn read<R>(mut fp: R) -> io::Result<()>
where R: Seek + Read {
    let tri = fp.read_u32::<LittleEndian>()?;
    let len = fp.read_u32::<LittleEndian>()? as usize;
    fp.seek(SeekFrom::Current(len as i64 * 4))?;
    println!("{:4} {} {}", len, tri, index::explain_packed(tri));
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut fp = io::BufReader::new(fs::File::open(&args[1]).unwrap());
    loop {
        read(&mut fp).unwrap();
    }
}
