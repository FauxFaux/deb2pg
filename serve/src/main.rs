extern crate byteorder;
extern crate catfight;
extern crate index;
extern crate iron;
extern crate logger;
extern crate lz4;
extern crate router;
#[macro_use]
extern crate serde_json;
extern crate stderrlog;
extern crate url;
extern crate persistent;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

use std::fs;
use std::io::Read as IoRead;
use std::io::Seek;
use std::io::SeekFrom;

use std::collections::HashSet;
use std::collections::HashMap;

use byteorder::{ByteOrder, LittleEndian};

use iron::prelude::*;
use iron::headers::ContentType;
use iron::status;
use router::Router;

use persistent::Read;
use r2d2::Pool;
pub struct AppDb;
impl iron::typemap::Key for AppDb {
    type Value = Pool<r2d2_postgres::PostgresConnectionManager>;
}

pub struct AppIndex;
impl iron::typemap::Key for AppIndex {
    type Value = index::find::Index<'static>;
}

enum Oid {
    Pos(i64),
    Hash(i64, i64, i64, i64),
}

fn oid_from_request(req: &mut Request) -> Option<Oid> {
    let mut param = req.extensions
        .get::<Router>()
        .unwrap()
        .find("bid")
        .unwrap()
        .chars();

    let id_type = param.next().expect("type");
    if ':' != param.next().expect("colon") {
        return None;
    }

    match id_type {
        'p' => {
            let parsed = param.collect::<String>().parse::<i64>();
            match parsed {
                Ok(val) => Some(Oid::Pos(val)),
                Err(_) => None,
            }
        }
        'h' => unreachable!(),
        _ => None,
    }
}

fn status(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((
        status::Ok,
        json!({
        "broken": true,
    }).to_string(),
    )))
}

fn blob(req: &mut Request) -> IronResult<Response> {
    let oid = oid_from_request(req).unwrap();

    let pool = req.get::<Read<AppDb>>().expect("persistent");
    let conn = pool.get().expect("pool");

    let h;
    let p;
    let len;

    match oid {
        Oid::Pos(pos) => {
            let stat = conn.prepare_cached("SELECT h0, h1, h2, h3, len FROM blob WHERE pos=$1")
                .unwrap();
            let result = stat.query(&[&pos]).unwrap();
            let row = result.get(0);
            h = hex_hash(compose(row.get(0), row.get(1), row.get(2), row.get(3)));

            p = format!("{}", pos as u64);

            len = row.get::<usize, i64>(4);
        }
        _ => return Ok(Response::with(status::BadRequest)),
    }

    Ok(Response::with((
        status::Ok,
        ContentType::json().0,
        json!({
        "ids": {
            "h": h,
            "p": p,
        },
        "len": len,
    }).to_string(),
    )))
}

fn cat(req: &mut Request) -> IronResult<Response> {
    let oid = oid_from_request(req).unwrap();
    match oid {
        Oid::Pos(i) => {
            let (name, off) = index::names::filename_for(i as u64);
            println!("{} {}", name, off);
            let mut fd = fs::File::open(format!("/mnt/data/t/{}", name)).unwrap();
            fd.seek(SeekFrom::Start(off as u64)).unwrap();
            let mut data = Vec::new();
            if let Some(mut record) = catfight::read_record(&mut fd).unwrap() {
                lz4::Decoder::new(&mut record.reader)
                    .expect("lz4")
                    .read_to_end(&mut data)
                    .expect("read");
                record.complete().expect("complete");
            } else {
                panic!()
            }
            Ok(Response::with(
                (status::Ok, ContentType::plaintext().0, data),
            ))
        }
        _ => unimplemented!(),
    }

}

fn paths(req: &mut Request) -> IronResult<Response> {
    let pos = if let Oid::Pos(pos) = oid_from_request(req).unwrap() {
        pos
    } else {
        unimplemented!()
    };

    let pool = req.get::<Read<AppDb>>().expect("persistent");
    let conn = pool.get().expect("pool");

    struct First {
        id: i64,
        paths: Vec<i64>,
    }

    let mut first = Vec::new();
    let mut path_ids = HashSet::new();

    let mut max_id = 0;

    let stat = conn.prepare_cached(
        "SELECT id, paths FROM file WHERE pos=$1 ORDER BY id LIMIT 501",
    ).unwrap();

    for row in stat.query(&[&pos]).unwrap().into_iter() {
        let row = First {
            id: row.get(0),
            paths: row.get(1),
        };

        // TODO: paging
        max_id = row.id;

        for id in &row.paths {
            path_ids.insert(*id);
        }

        first.push(row);
    }

    let stat = conn.prepare_cached("SELECT id, path FROM path_component WHERE id = ANY ($1)")
        .unwrap();

    let mut id_paths: HashMap<i64, String> = HashMap::with_capacity(path_ids.len());

    for row in stat.query(&[&path_ids.into_iter().collect::<Vec<i64>>()])
        .unwrap()
        .into_iter()
    {
        id_paths.insert(row.get(0), row.get(1));
    }

    let paths = first
        .into_iter()
        .map(|f| {
            f.paths.iter().map(|id| id_paths[id].to_string()).collect()
        })
        .collect::<Vec<Vec<String>>>();

    Ok(Response::with((
        status::Ok,
        ContentType::json().0,
        json!({
            "paths": paths,
        }).to_string(),
    )))
}

fn tri_num(req: &mut Request) -> IronResult<Response> {
    let tri = req.extensions
        .get::<Router>()
        .unwrap()
        .find("num")
        .unwrap()
        .parse::<u32>()
        .unwrap();

    let index = req.get::<Read<AppIndex>>().expect("persistent");

    let docs = index.documents_for_tri(tri);
    Ok(Response::with((
        status::Ok,
        ContentType::json().0,
        json!({
            "docs": docs,
        }).to_string(),
    )))
}

fn search(req: &mut Request) -> IronResult<Response> {
    let term: String = req.extensions
        .get::<Router>()
        .unwrap()
        .find("term")
        .unwrap()
        .to_string();

    let term = url::percent_encoding::percent_decode(term.as_bytes())
        .decode_utf8()
        .expect("query")
        .to_string();

    let index = req.get::<Read<AppIndex>>().expect("persistent");
    let search = index.documents_for_search(&term);
    Ok(Response::with((
        status::Ok,
        ContentType::json().0,
        json!({
            "docs": search.docs,
            "grepped": search.grepped,
        }).to_string(),
    )))
}

fn compose(h0: i64, h1: i64, h2: i64, h3: i64) -> [u8; 256 / 8] {
    let mut hash = [0; 256 / 8];
    LittleEndian::write_i64(&mut hash[0..8], h0);
    LittleEndian::write_i64(&mut hash[8..16], h1);
    LittleEndian::write_i64(&mut hash[16..24], h2);
    LittleEndian::write_i64(&mut hash[24..32], h3);
    hash
}

fn hex_hash(hash: [u8; 256 / 8]) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

fn main() {
    // 2: info
    // 3: debug
    stderrlog::new().verbosity(2).init().unwrap();

    let manager = r2d2_postgres::PostgresConnectionManager::new(
        "postgres://faux@%2Frun%2Fpostgresql",
        r2d2_postgres::TlsMode::None,
    ).unwrap();
    let pool = r2d2::Pool::builder().max_size(8).build(manager).unwrap();
    std::mem::drop(pool.get().unwrap());

    let index = {
        let mut paths = Vec::new();
        for file in fs::read_dir("/mnt/data/t").unwrap() {
            let path = file.unwrap().path();
            if path.extension().unwrap().to_str().unwrap() != "idx" {
                continue;
            }

            paths.push(path);
        }

        println!("{} paths found; going to open index...", paths.len());
        index::find::Index::open(paths).unwrap()
    };

    println!("index loaded");

    let mut router = Router::new();
    router.get("/ds/status", status, "status");
    router.get("/ds/blob/:bid", blob, "blob-details");
    router.get("/ds/cat/:bid", cat, "blob-contents");
    router.get("/ds/paths/:bid", paths, "paths");
    router.get("/ds/search/:term", search, "search");

    // Debug:
    router.get("/ds/trinum/:num", tri_num, "trinum");

    let (logger_before, logger_after) = logger::Logger::new(None);

    let mut chain = Chain::new(router);
    chain.link_before(logger_before);
    chain.link(Read::<AppDb>::both(pool));
    chain.link(Read::<AppIndex>::both(index));
    chain.link_after(logger_after);

    Iron::new(chain).http("127.0.01:6918").unwrap();
}
