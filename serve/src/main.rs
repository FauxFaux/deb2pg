extern crate iron;
extern crate logger;
extern crate router;
#[macro_use]
extern crate serde_json;
extern crate stderrlog;
extern crate persistent;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

use iron::prelude::*;
use iron::status;
use router::Router;

use persistent::Read;
use r2d2::Pool;
pub struct AppDb;
impl iron::typemap::Key for AppDb { type Value = Pool<r2d2_postgres::PostgresConnectionManager>; }

fn status(req: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::Ok, json!({
        "broken": true,
    }).to_string())))
}

fn blob(req: &mut Request) -> IronResult<Response> {
    let id: String;
    let id_type;
    {
        let mut param = req.extensions.get::<Router>().expect("param")
            .find("id").expect("id")
            .chars();

        id_type = param.next().expect("type");
        if ':' != param.next().expect("colon") {
            return Ok(Response::with(status::BadRequest));
        }

        id = param.collect();
    }

    let pool = req.get::<Read<AppDb>>().expect("persistent");
    let conn = pool.get().expect("pool");

    match id_type {
        'p' => {
            let stat = conn.prepare_cached("SELECT h0, h1, h2, h3, len FROM blob WHERE pos=$1").unwrap();
            let result = stat.query(&[&id.parse::<i64>().unwrap()]).unwrap();
            let row = result.get(0);
            Ok(Response::with((status::Ok, json!({
                 "h0": row.get::<usize, i64>(0),
            }).to_string())))
        }
        _ => Ok(Response::with(status::BadRequest))
    }
}

fn main() {
    // 2: info
    // 3: debug
    stderrlog::new().verbosity(2).init().unwrap();

    let manager = r2d2_postgres::PostgresConnectionManager::new("postgres://faux@%2Frun%2Fpostgresql", r2d2_postgres::TlsMode::None).unwrap();
    let config = r2d2::Config::builder().pool_size(8).build();
    let pool = r2d2::Pool::new(config, manager).unwrap();

    let mut router = Router::new();
    router.get("/ds/status", status, "status");
    router.get("/ds/blob/:id", blob, "blob-details");

    let (logger_before, logger_after) = logger::Logger::new(None);

    let mut chain = Chain::new(router);
    chain.link_before(logger_before);
    chain.link(Read::<AppDb>::both(pool));
    chain.link_after(logger_after);

    Iron::new(chain).http("localhost:6918").unwrap();
}
