extern crate byteorder;
#[macro_use]
extern crate error_chain;
extern crate fs2;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate more_asserts;
extern crate postgres;
extern crate serde_json;
extern crate sha2;
extern crate splayers;
extern crate tempfile;
extern crate tempfile_fast;
extern crate zstd;

use std::env;
use std::fs;

use std::collections::hash_map;
use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};

mod dicts;
mod temps;
mod store;

use errors::*;
use temps::TempFile;

fn run() -> Result<()> {
    assert_eq!(4, env::args().len());

    let package_name = env::args().nth(1).unwrap();
    let package_version = env::args().nth(2).unwrap();
    let source = env::args().nth(3).unwrap();

    let out_dir = "/mnt/data/t".to_string();
    let container_info = serde_json::to_value(&hashmap! {
        "type" => "debian",
        "package" => &package_name,
        "version" => &package_version,
    })?;

    // Weird lifetime alarm: paths become invalid when this is dropped.
    let temp_files = splayers::Unpack::unpack_into(source, &out_dir)?;

    let data_conn = connect()?;

    let mut blobs = HashMap::new();

    let meta_conn = connect()?;
    let meta_tran = meta_conn.transaction()?;

    let container_id: i64 = meta_tran
        .query(
            "
INSERT INTO container (info) VALUES ($1) RETURNING id
"
                .trim(),
            &[&container_info],
        )
        .chain_err(|| "inserting container info")?
        .iter()
        .next()
        .unwrap()
        .get(0);

    let insert_file = meta_tran.prepare(
        "
INSERT INTO file (container, pos, paths) VALUES ($1, $2, $3)
"
            .trim(),
    )?;

    let mut store = store::ShardedStore::new(out_dir);

    match temp_files.status() {
        &splayers::Status::Success(ref entries) => loopy(
            entries,
            &mut blobs,
            &mut store,
            data_conn,
            &[],
            insert_file,
            container_id,
        )?,
        other => bail!("root must be unpackable, not {:?}", other),
    }

    meta_tran.commit()?;

    Ok(())
}

fn loopy(
    entries: &[splayers::Entry],
    blobs: &mut HashMap<[u8; 256 / 8], i64>,
    store: &mut store::ShardedStore,
    data_conn: postgres::Connection,
    path: &[i64],
    insert_file: postgres::stmt::Statement,
    container_id: i64,
) -> Result<()> {
    let names = write_names(
        &data_conn,
        entries
            .into_iter()
            .map(|entry| String::from_utf8_lossy(&entry.local.path)),
    )?;
    for entry in entries {
        if entry.local.temp.is_none() {
            continue;
        }

        let mut file = match temps::hash_compress_write_from_reader(
            fs::File::open(entry.local.temp.as_ref().unwrap())?,
            &String::from_utf8_lossy(&entry.local.path),
            store.locality(),
        )? {
            Some(x) => x,
            None => continue,
        };
        let pos: i64 = match blobs.entry(file.hash) {
            hash_map::Entry::Vacant(storable) => {
                *storable.insert(maybe_store(store, file, data_conn.transaction()?)?)
            }
            hash_map::Entry::Occupied(occupied) => *occupied.get(),
        };

        // TODO: let _ = fs::remove_file(&file.name);

        let mut path = path.to_vec();
        path.insert(
            0,
            names[&String::from_utf8_lossy(&entry.local.path).to_string()],
        );

        insert_file.execute(&[&container_id, &pos, &path])?;
    }

    Ok(())
}

fn connect() -> Result<postgres::Connection> {
    postgres::Connection::connect(
        "postgres://faux@%2Frun%2Fpostgresql",
        postgres::TlsMode::None,
    ).chain_err(|| "connecting to postgres")
}

/// Store the supplied `TempFile` in the appropriate shard in the `shard_root`,
/// if it is not already present in the database.
fn maybe_store(
    store: &mut store::ShardedStore,
    file: TempFile,
    curr: postgres::transaction::Transaction,
) -> Result<i64> {
    // Postgres doesn't do unsigned.
    assert_le!(file.len, std::i64::MAX as u64);
    let size = file.len as i64;

    // Firstly, if it's already there, we're done!
    let (h0, h1, h2, h3) = decompose(file.hash);
    if let Some(pos) = fetch(&curr, h0, h1, h2, h3, size)? {
        return Ok(pos);
    }

    // Otherwise, lock the db, and insert
    curr.prepare_cached(
        "
SELECT pg_advisory_lock(18787)
"
            .trim(),
    )?
        .execute(&[])?;

    let done = curr.prepare_cached(
        "
INSERT INTO blob (h0, h1, h2, h3, len)
SELECT $1, $2, $3, $4, $5
WHERE NOT EXISTS (SELECT TRUE FROM blob WHERE h0=$1 AND h1=$2 AND h2=$3 AND h3=$4 AND len=$5)
"
            .trim(),
    )?
        .execute(&[&h0, &h1, &h2, &h3, &size])?;

    curr.prepare_cached(
        "
SELECT pg_advisory_unlock(18787)
"
            .trim(),
    )?
        .execute(&[])?;

    if done == 0 {
        // we didn't insert the row, so no need to do anything
        return Ok(fetch(&curr, h0, h1, h2, h3, size)?.expect("we just checked it was there..."));
    }

    let pos = store.store(file.file, || {
        Ok(curr.prepare_cached("SELECT nextval('loose_blob_seq')")?
            .query(&[])?
            .iter()
            .next()
            .unwrap()
            .get::<usize, i64>(0) as u64)
    })? as i64;

    curr.prepare_cached(
        "
UPDATE blob SET pos=$1 WHERE h0=$2 AND h1=$3 AND h2=$4 AND h3=$5 AND len=$6
"
            .trim(),
    )?
        .execute(&[&pos, &h0, &h1, &h2, &h3, &size])?;

    curr.commit()?;
    Ok(pos)
}

fn fetch(
    curr: &postgres::transaction::Transaction,
    h0: i64,
    h1: i64,
    h2: i64,
    h3: i64,
    len: i64,
) -> Result<Option<i64>> {
    let statement = curr.prepare_cached(
        "
SELECT pos FROM blob WHERE h0=$1 AND h1=$2 AND h2=$3 AND h3=$4 AND len=$5
"
            .trim(),
    )?;
    let result = statement.query(&[&h0, &h1, &h2, &h3, &len])?;
    Ok(result.iter().next().map(|row| row.get::<usize, i64>(0)))
}

fn decompose(hash: [u8; 256 / 8]) -> (i64, i64, i64, i64) {
    (
        LittleEndian::read_i64(&hash[0..8]),
        LittleEndian::read_i64(&hash[8..16]),
        LittleEndian::read_i64(&hash[16..24]),
        LittleEndian::read_i64(&hash[24..32]),
    )
}

fn write_names<'i, I>(conn: &postgres::Connection, wat: I) -> Result<HashMap<String, i64>>
where
    I: Iterator<Item = ::std::borrow::Cow<'i, str>>,
{
    let write = conn.prepare(
        "
INSERT INTO path_component (path) VALUES ($1)
ON CONFLICT DO NOTHING
RETURNING id"
            .trim(),
    )?;
    let read_back = conn.prepare("SELECT id FROM path_component WHERE path=$1")?;

    let mut map: HashMap<String, i64> = HashMap::new();
    for path in wat {
        let path = path.to_string();
        if let hash_map::Entry::Vacant(vacancy) = map.entry(path.to_string()) {
            let id: i64 = match write.query(&[&path])?.iter().next() {
                Some(row) => row.get(0),
                None => match read_back.query(&[&path])?.iter().next() {
                    Some(row) => row.get(0),
                    None => bail!(ErrorKind::InvalidState(format!(
                        "didn't write and didn't find '{}'",
                        path
                    ),)),
                },
            };

            assert_ge!(id, 0);
            vacancy.insert(id);
        }
    }
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
            Pg(::postgres::error::Error);
            SerdeJson(::serde_json::Error);
            Splayers(::splayers::Error);
        }
    }
}
