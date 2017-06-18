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

use std::env;
use std::fs;

use std::collections::hash_map;
use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};

use temps::TempFile;

use errors::*;

fn run() -> Result<i32> {
    assert_eq!(3, env::args().len());

    // TODO: JSON injection
    let package_name = env::args().nth(1).unwrap();
    let package_version = env::args().nth(2).unwrap();

    let out_dir = "/var/ftmp/t/".to_string();
    let container_info = format!("{{'type': 'debian', 'package': '{}', 'version': '{}'}}",
                                 package_name, package_version);

    let temp_files = temps::read(out_dir.as_str())?;

    let data_conn = connect()?;

    let name_ids = write_names(&data_conn, &temp_files.iter()
        .flat_map(|file| file.header.paths.iter())
        .collect::<Vec<&String>>())?;

    let mut blobs = HashMap::with_capacity(temp_files.len());

    let meta_conn = connect()?;
    let meta_tran = meta_conn.transaction()?;

    let container_id: i64 = meta_tran.query("
INSERT INTO container (info) VALUES (to_jsonb($1::text)) RETURNING id
", &[&container_info.to_string()]).chain_err(|| "inserting container info")?
        .iter().next().unwrap().get(0);

    let insert_file = meta_tran.prepare("
INSERT INTO file (container, pos, paths) VALUES ($1, $2, $3)
")?;
    for file in temp_files {
        let pos: u64 = match blobs.entry(file.hash) {
            hash_map::Entry::Vacant(storable) =>
                *storable.insert(maybe_store(out_dir.as_str(), &file, data_conn.transaction()?)?),
            hash_map::Entry::Occupied(occupied) => *occupied.get(),
        };

        let path = file.header.paths.iter().map(|part| name_ids[part]).collect::<Vec<i64>>();
        insert_file.execute(&[&container_id, &(pos as i64), &path])?;
    }

    meta_tran.commit()?;

    Ok(0)
}

fn connect() -> Result<postgres::Connection> {
    postgres::Connection::connect("postgres://faux@%2Frun%2Fpostgresql", postgres::TlsMode::None)
        .chain_err(|| "connecting to postgres")
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
    curr.prepare_cached("
SELECT pg_advisory_lock(18787)
")?.execute(&[])?;

    let done = curr.prepare_cached("
INSERT INTO blob (h0, h1, h2, h3, len)
SELECT $1, $2, $3, $4, $5
WHERE NOT EXISTS (SELECT TRUE FROM blob WHERE h0=$1 AND h1=$2 AND h2=$3 AND h3=$4 AND len=$5)
")?.execute(&[&h0, &h1, &h2, &h3, &size])?;

    curr.prepare_cached("
SELECT pg_advisory_unlock(18787)
")?.execute(&[])?;

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

    curr.prepare_cached("
UPDATE blob SET pos=$1 WHERE h0=$2 AND h1=$3 AND h2=$4 AND h3=$5 AND len=$6
")?.execute(&[&(pos as i64), &h0, &h1, &h2, &h3, &size])?;

    curr.commit()?;
    fs::remove_file(&file.name)?;
    Ok(pos)
}

fn fetch(curr: &postgres::transaction::Transaction, h0: i64, h1: i64, h2: i64, h3: i64, len: i64) -> Result<Option<u64>> {
    let statement = curr.prepare_cached("
SELECT pos FROM blob WHERE h0=$1 AND h1=$2 AND h2=$3 AND h3=$4 AND len=$5
")?;
    let result = statement.query(&[&h0, &h1, &h2, &h3, &len])?;
    Ok(result.iter().next().map(|row| row.get::<usize, i64>(0) as u64))
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
