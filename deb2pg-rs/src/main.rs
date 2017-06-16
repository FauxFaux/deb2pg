extern crate base32;
extern crate byteorder;
extern crate ci_capnp;
#[macro_use]
extern crate error_chain;
extern crate libc;
extern crate lz4;
extern crate num_cpus;
extern crate sha2;
extern crate tempfile;
extern crate thread_pool;

mod catfight;
mod copy;
//mod simplify_path;
mod temps;

use errors::*;

fn run() -> Result<i32> {
    let temp_files = temps::read(&"t".to_string())?;
    for file in temp_files {
        println!("{}: {:?}", file.name, file.header.paths);
    }
    unimplemented!();
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
        }
    }
}
