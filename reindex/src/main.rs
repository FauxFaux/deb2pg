extern crate catfight;
extern crate index;
extern crate postgres;
extern crate lz4;

use std::env;
use std::fs;

use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fmt::Write;

fn main() {
    let args: Vec<String> = env::args().collect();
    let conn = postgres::Connection::connect(
        "postgres://faux@%2Frun%2Fpostgresql",
        postgres::TlsMode::None).unwrap();
    let tran = conn.transaction().unwrap();
    let stat = tran.prepare("COPY tri FROM STDIN").unwrap();

    let mut fp = fs::File::open(&args[1]).unwrap();
    fp.seek(SeekFrom::Start(16)).unwrap();
    let mut write = String::new();
    loop {
        write.truncate(0);
        let pos = fp.seek(SeekFrom::Current(0)).unwrap();
        if let Some(mut entry) = catfight::read_record(&mut fp).unwrap() {
            // len is the compressed length, but better than zero
            let mut buf = Vec::with_capacity(entry.len as usize);
            lz4::Decoder::new(&mut entry.reader).unwrap().read_to_end(&mut buf).unwrap();

            for t in index::trigrams_full(&String::from_utf8_lossy(&buf)) {
//                stat.execute(&[&(pos as i64), &(t as i32)]).unwrap();
                write!(write, "{}\t{}\n", pos, t).unwrap();
            }

            entry.complete().unwrap();

            stat.copy_in(&[], &mut write.as_bytes()).unwrap();
        } else {
            break;
        }

        // Basically random, although does need to be mod 16.
        if 0 == (pos % 16 * 1024) {
            println!("{:0.2}", pos as f32 / 1e9 * 100.0f32);
        }
    }

    tran.commit().unwrap();
}
