extern crate byteorder;
#[macro_use]
extern crate error_chain;
extern crate libc;
extern crate peeky_read;

mod catfight;
mod copy;

pub use catfight::read_record;
pub use catfight::flock;
pub use catfight::unlock_flock;
pub use catfight::writey_write;

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
