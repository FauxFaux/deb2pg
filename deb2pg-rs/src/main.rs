extern crate base32;
extern crate byteorder;
extern crate ci_capnp;
#[macro_use]
extern crate error_chain;
extern crate libc;
extern crate lz4;
extern crate num_cpus;
extern crate postgres;
extern crate sha2;
extern crate tempfile;
extern crate thread_pool;

mod catfight;
mod copy;
//mod simplify_path;
mod temps;

use std::fs;

use std::collections::hash_map;
use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};

use temps::TempFile;

use errors::*;

fn run() -> Result<i32> {
    let out_dir = "/var/ftmp/t/".to_string();

    let temp_files = temps::read(out_dir.as_str())?;

    let conn = postgres::Connection::connect("postgres://faux@%2Frun%2Fpostgresql", postgres::TlsMode::None)?;
    let name_ids = write_names(&conn, &temp_files.iter()
        .flat_map(|file| file.header.paths.iter())
        .collect::<Vec<&String>>());

    let mut blobs = HashMap::with_capacity(temp_files.len());

    for file in temp_files {
        let pos: u64 = match blobs.entry(file.hash) {
            hash_map::Entry::Vacant(storable) =>
                *storable.insert(maybe_store(out_dir.as_str(), &file, conn.transaction()?)?),
            hash_map::Entry::Occupied(occupied) => *occupied.get(),
        };
    }

    Ok(0)
}

/// Store the supplied `TempFile` in the appropriate shard in the `shard_root`,
/// if it is not already present in the database.
fn maybe_store(
    shard_root: &str,
    file: &TempFile,
    curr: postgres::transaction::Transaction) -> Result<u64> {

    // Postgres doesn't do unsigned.
    assert!(file.header.len <= std::i64::MAX as u64);
    let size = file.header.len as i64;

    // Firstly, if it's already there, we're done!
    let (h0, h1, h2, h3) = decompose(file.hash);
    if let Some(pos) = fetch(&curr, h0, h1, h2, h3, size)? {
        return Ok(pos);
    }

    // Otherwise, lock the db, and insert
    curr.execute("
SELECT pg_advisory_lock(18787)
", &[])?;

    let done = curr.execute("
INSERT INTO blob (h0, h1, h2, h3, len)
SELECT %s, %s, %s, %s, %s
WHERE NOT EXISTS (SELECT TRUE FROM blob WHERE h0=%s AND h1=%s AND h2=%s AND h3=%s AND len=%s)
", &[&h0, &h1, &h2, &h3, &size, &h0, &h1, &h2, &h3, &size])?;

    curr.execute("
SELECT pg_advisory_unlock(18787)
", &[])?;

    if done == 0 {
        // we didn't insert the row, so no need to do anything
        fs::remove_file(&file.name)?;
        return Ok(fetch(&curr, h0, h1, h2, h3, size)?.expect("we just checked it was there..."));
    }

    let shard_no = make_shard_no(file.header.len);
    let shard_name = format!("{}-{}", if file.text { "text" } else { "bin" }, shard_no);
    let shard_id = shard_no - 2 + if file.text { 8 } else { 0 };

    let pos = (shard_id as u64) + catfight::store(
        1024 * 1024 * 1024,
        file.name.as_str(),
        format!("{}/{}", shard_root, shard_name).as_str(),
        &temps::encode_hash(&file.hash))?;

    curr.execute("
UPDATE blob SET pos=%s WHERE h0=%s AND h1=%s AND h2=%s AND h3=%s AND len=%s
", &[&(pos as i64), &h0, &h1, &h2, &h3, &size])?;

    curr.commit()?;
    fs::remove_file(&file.name)?;
    Ok(pos)
}

fn fetch(curr: &postgres::transaction::Transaction, h0: i64, h1: i64, h2: i64, h3: i64, len: i64) -> Result<Option<u64>> {
    Ok(curr.query("
SELECT pos FROM blob WHERE h0=%s AND h1=%s AND h2=%s AND h3=%s AND len=%s
", &[&h0, &h1, &h2, &h3, &len])?.iter().next().map(|row| row.get::<usize, i64>(0) as u64))
}

fn make_shard_no(size: u64) -> u8 {
    use std::cmp::{min, max};
    min(9, max(2, (size as f64).log10() as u64)) as u8
}

fn decompose(hash: [u8; 256 / 8]) -> (i64, i64, i64, i64) {
    (
        LittleEndian::read_i64(&hash[0..8]),
        LittleEndian::read_i64(&hash[8..16]),
        LittleEndian::read_i64(&hash[16..24]),
        LittleEndian::read_i64(&hash[24..32]),
    )
}

// TODO: iterator
fn write_names(conn: &postgres::Connection, wat: &[&String]) -> Result<HashMap<String, i64>> {
    let tran = conn.transaction()?;
    let write = tran.prepare("
INSERT INTO path_component (path) VALUES ($1)
ON CONFLICT DO NOTHING
RETURNING id")?;
    let read_back = tran.prepare("SELECT id FROM path_component WHERE path=$1")?;

    let mut map: HashMap<String, i64> = HashMap::new();
    for path in wat {
        if let hash_map::Entry::Vacant(vacancy) = map.entry(path.to_string()) {
            let id: i64 = match write.query(&[&path])?.iter().next() {
                Some(row) => row.get(0),
                None => match read_back.query(&[&path])?.iter().next() {
                    Some(row) => row.get(0),
                    None => bail!(ErrorKind::InvalidState(format!("didn't write and didn't find '{}'", path))),
                }
            };

            assert!(id >= 0);
            vacancy.insert(id);
        }
    }
    tran.commit()?;
    Ok(map)
}

quick_main!(run);

mod errors {
    error_chain! {
        errors {
            InvalidState(msg: String) {
                description("assert!")
                display("invalid state: {}", msg)
            }
        }

        foreign_links {
            Io(::std::io::Error);
            PgConnect(::postgres::error::ConnectError);
            Pg(::postgres::error::Error);
        }
    }
}
