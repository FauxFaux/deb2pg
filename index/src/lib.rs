#![feature(decode_utf8)]
#![feature(io)]

extern crate bit_set;
extern crate byteorder;
extern crate libc;
extern crate lz4;
extern crate regex_syntax;

mod indexer;
mod search;
mod tri;
