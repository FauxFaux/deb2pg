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

use std::collections::hash_map;
use std::collections::HashMap;

use errors::*;

fn run() -> Result<i32> {
    let temp_files = temps::read(&"/var/ftmp/".to_string())?;

    let conn = postgres::Connection::connect("postgres://faux@%2Frun%2Fpostgresql", postgres::TlsMode::None)?;
    let map = write_names(&conn, &temp_files.iter()
        .flat_map(|file| file.header.paths.iter())
        .collect::<Vec<&String>>());

    Ok(0)
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
