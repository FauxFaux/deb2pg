extern crate bit_set;
extern crate byteorder;
extern crate catfight;
#[macro_use]
extern crate error_chain;
extern crate libc;
extern crate lz4;
extern crate memmap;
extern crate regex_syntax;
extern crate twoway;

pub mod find;
pub mod names;
mod search;
mod tri;

pub use tri::trigrams_full;
pub use tri::explain_packed;

#[cfg(test)]
mod tests;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
        }
    }
}
