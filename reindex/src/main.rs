extern crate catfight;
extern crate index;
extern crate postgres;
extern crate lz4;

use std::env;
use std::fs;

use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

fn main() {
    let args: Vec<String> = env::args().collect();
    let conn = postgres::Connection::connect(
        "postgres://faux@%2Frun%2Fpostgresql",
        postgres::TlsMode::None).unwrap();
    let tran = conn.transaction().unwrap();
    let stat = tran.prepare("INSERT INTO tri (pos, tri) VALUES ($1, $2)").unwrap();

    let mut fp = fs::File::open(&args[1]).unwrap();
    fp.seek(SeekFrom::Start(16)).unwrap();
    loop {
        let pos = fp.seek(SeekFrom::Current(0)).unwrap();
        if let Some(mut entry) = catfight::read_record(&mut fp).unwrap() {
            // len is the compressed length, but better than zero
            let mut buf = Vec::with_capacity(entry.len as usize);
            lz4::Decoder::new(&mut entry.reader).unwrap().read_to_end(&mut buf).unwrap();

            for t in index::trigrams_full(&String::from_utf8_lossy(&buf)) {
                stat.execute(&[&(pos as i64), &(t as i32)]).unwrap();
            }

            entry.complete().unwrap();
        } else {
            break;
        }
    }

    tran.commit().unwrap();
}
