extern crate byteorder;
extern crate getopts;
extern crate libc;
extern crate rand;

mod copy;

// normal, sane imports:
use std::env;
use std::io;
use std::io::prelude::*;
use std::fs;
use std::fs::File;

// from crates:
use getopts::Options;

// magic method-adding imports:
use std::os::unix::io::AsRawFd;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

fn unarchive(root: &str, block_size: u64, offset: u64) -> Result<(), io::Error> {
    let target_file_id: u64 = offset / block_size;
    let target_file_offset = offset % block_size;

    let target_path = format!("{}.{:022}", root, target_file_id);
    let mut fd = try!(File::open(target_path));
    let file_len = try!(fd.seek(io::SeekFrom::End(0)));

    try!(fd.seek(io::SeekFrom::Start(target_file_offset)));
    let end = try!(fd.read_u64::<BigEndian>());
    let extra_len = try!(fd.read_u64::<BigEndian>());

    assert!(end >= 8 + 8,
            "there isn't even a header, invalid offset?");

    assert!(extra_len < std::i64::MAX as u64,
            "extra length is far too large, invalid offset?");

    assert!(extra_len <= end - 8 - 8,
            "too much extra data for record; invalid offset?");

    assert!(target_file_offset + end <= file_len,
            "record extends beyond end of file; invalid offset?");

    try!(fd.seek(io::SeekFrom::Current(extra_len as i64)));

    try!(copy::copy_file(&mut fd, &mut io::stdout(), end - extra_len - 8 - 8));

    return Ok(());
}

fn create_hint_temp_file(hint_path: &str, to_write: &str) -> Result<String, io::Error> {
    for _ in 0..50_000 {
        let unique: u64 = rand::random();
        let temp_file = format!("{}.{}~", hint_path, unique);
        match std::os::unix::fs::symlink(&to_write, &temp_file) {
            Ok(()) => return Ok(temp_file),
            Err(err) => {
                if let Some(code) = err.raw_os_error() {
                    if libc::EEXIST == code {
                        continue;
                    }
                }
                return Err(err);
            },
        }
    }
    return Err(io::Error::new(io::ErrorKind::Other, "couldn't race temporary file creation"));
}

fn write_hint(hint_path: &str, val: u64) -> Result<(), std::io::Error> {
    let to_write = format!("/{}", val);
    let temp_file = try!(create_hint_temp_file(hint_path, &to_write));
    return fs::rename(&temp_file, hint_path).or_else(|mv_err| {
        if let Err(rm_err) = fs::remove_file(&temp_file) {
            print!("warning: couldn't remove temporary file '{}': {}\n", temp_file, rm_err);
        }
        return Err(mv_err);
    });
}

fn read_hint(hint_path: &str) -> u64 {
    let content = match std::fs::read_link(hint_path) {
        Ok(contents) => contents,
        Err(err) => {
            if let Some(code) = err.raw_os_error() {
                if libc::ENOENT == code {
                    if let Err(e) = write_hint(hint_path, 0) {
                        print!("warning: couldn't create initial hint file: {}\n", e);
                    }
                    return 0;
                }
            }
            print!("warning: hint file problem: {}\n", err);
            return 0;
        },
    };

    let without_leading_slash = &(content.to_string_lossy()[1..]);
    return match without_leading_slash.parse() {
        Ok(x) => x,
        Err(e) => {
            print!("warning: invalid hint file, ignoring: {}\n", e);
            return 0;
        }
    };
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("\
    Usage: {0} [options] [-e extra-data] output-prefix input\n\
    Usage: {0} [options] -u offset output-prefix\n\
    ", program);
    print!("{}", opts.usage(&brief));
}

/// @return true if locked, false if locking would block, error if something went wrong
fn non_blocking_flock(what: &File) -> Result<bool, io::Error> {
    unsafe {
        let ret = libc::flock(what.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB);
        if 0 != ret {
            let failure = io::Error::last_os_error();
            if let Some(code) = failure.raw_os_error() {
                if libc::EWOULDBLOCK == code {
                    return Ok(false);
                }
            }
            return Err(failure)
        }
    }
    return Ok(true)
}

fn unlock_flock(what: &File) -> Result<(), io::Error> {
    unsafe {
        let ret = libc::flock(what.as_raw_fd(), libc::LOCK_UN);
        if 0 != ret {
            return Err(io::Error::last_os_error())
        }
    }
    return Ok(())
}

#[derive(PartialEq)]
enum Operation {
    Archive,
    Unarchive,
}

fn real_main() -> u8 {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("b", "block-size", "overflow point for file parts", "BYTES");
    opts.optopt("e", "extra", "extra metadata to include", "DATA");
    opts.optopt("u", "unarchive", "extract file from this offset", "OFFSET");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return 2;
    }

    if matches.opt_present("e") && matches.opt_present("u") {
        print!("-e and -u don't make sense together\n");
        return 3;
    }

    let blocksize: u64 = match matches.opt_str("b") {
        Some(x) => x.parse().unwrap(),
        None => 1 * 1024 * 1024 * 1024
    };

    if blocksize <= 16 {
        print!("blocksize must be >16\n");
        return 3;
    }

    let op = if matches.opt_present("u") {
        Operation::Unarchive
    } else {
        Operation::Archive
    };

    if Operation::Unarchive == op {
        let offset: u64 = match matches.opt_str("u") {
            Some(x) => x.parse().unwrap(),
            None => unreachable!(),
        };

        if 1 != matches.free.len() {
            print_usage(&program, opts);
            return 2;
        }

        unarchive(matches.free[0].as_str(), blocksize, offset).unwrap();
        return 0;
    }

    if 2 != matches.free.len() {
        print_usage(&program, opts);
        return 2;
    }

    let dest_root = matches.free[0].clone();
    let src_path = matches.free[1].as_str();

    let extra = match matches.opt_str("e") {
        Some(x) => x,
        None => String::from(""),
    };

    // read-only by default
    let mut src = match File::open(src_path) {
        Ok(x) => x,
        Err(e) => {
            print!("src file problem: {}: {}\n", src_path, e);
            return 4;
        }
    };

    let src_len: u64 = match fs::metadata(src_path) {
        Ok(x) => x.len(),
        Err(e) => {
            print!("src file doesn't stat: {}: {}\n", src_path, e);
            return 5;
        }
    };

    let hint_path = dest_root.clone() + ".hint";
    let hint: u64 = read_hint(hint_path.as_str());
    let mut skipped_due_to_locking = false;

    for target_num in 0..std::u64::MAX {
        let target_path = format!("{}.{:022}", dest_root, target_num);
        let mut fd = std::fs::OpenOptions::new()
            .write(true).create(true)
            .open(target_path.as_str()).unwrap();

        if !non_blocking_flock(&fd).unwrap() {
            skipped_due_to_locking = true;
            continue;
        }

        let mut seek: u64 = fd.seek(io::SeekFrom::End(0)).unwrap();

        if 0 == seek {
            // we locked a new file, write a header
            fd.write_all(b"cf1\0\0\0\0\0").unwrap();
            fd.write_u64::<BigEndian>(target_num).unwrap();
            seek = 16;
        }

        assert!(seek + 16 < (std::i64::MAX as u64));

        if 0 != (seek % 16) {
            let adjustment: i64 = 16 - (seek % 16) as i64;
            seek = fd.seek(std::io::SeekFrom::Current(adjustment)).unwrap();
        }

        if seek >= blocksize {
            continue;
        }

        let extra_len: u64 = extra.len() as u64;
        let record_end = 8 + 8 + src_len + extra_len;
        fd.write_u64::<BigEndian>(record_end).unwrap();
        fd.write_u64::<BigEndian>(extra_len).unwrap();
        fd.write_all(extra.as_bytes()).unwrap();
        fd.flush().unwrap();

        fd.set_len(seek + record_end).unwrap();
        unlock_flock(&fd).unwrap();

        copy::copy_file(&mut src, &mut fd, src_len).unwrap();

        print!("{}\n", target_num * blocksize + seek);

        if !skipped_due_to_locking && target_num > hint {
            if let Err(e) = write_hint(&hint_path, target_num) {
                print!("warning: couldn't update hint: {}\n", e);
            }
        }

        break;
    }

    return 0;
}

fn main() {
    std::process::exit(real_main() as i32);
}
