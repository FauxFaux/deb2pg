// Expecting this to be stabilised really soon,
// and significantly simplifies the implementation of Drop.
// Totally possible to re-write in terms of into_inner(), I think.
#![feature(more_io_inner_methods)]

extern crate byteorder;
#[macro_use]
extern crate error_chain;
extern crate peeky_read;
extern crate libc;

mod catfight;
mod copy;

pub use catfight::read_record;
pub use catfight::store;

pub use errors::{Error, ErrorKind, Result};

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
