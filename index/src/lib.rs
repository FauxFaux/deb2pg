#![feature(decode_utf8)]
#![feature(io)]

extern crate bit_set;
extern crate byteorder;
#[macro_use]
extern crate error_chain;
extern crate libc;
extern crate lz4;
extern crate regex_syntax;

mod indexer;
mod search;
mod tri;

pub use tri::trigrams_full;

#[cfg(test)]
mod tests;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
        }
    }
}
