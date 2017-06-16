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
mod temps;

use errors::*;

fn run() -> Result<i32> {
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
