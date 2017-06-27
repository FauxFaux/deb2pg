use libc;

use std;
use std::io;
use std::fs::File;
use std::os::unix::io::{RawFd, AsRawFd};

enum CopyFailure {
    Unsupported,
    Errno(io::Error),
}

pub trait MyRawFd {
    fn my_raw_fd(&self) -> RawFd;
}

impl MyRawFd for io::Stdout {
    fn my_raw_fd(&self) -> RawFd {
        1
    }
}

impl MyRawFd for File {
    fn my_raw_fd(&self) -> RawFd {
        self.as_raw_fd()
    }
}

fn try_sendfile(src: &File, dest: &MyRawFd, len: u64) -> Result<(), CopyFailure> {
    let mut remaining = len;
    while remaining > 0 {
        unsafe {
            let offset: *mut i64 = std::ptr::null_mut();
            let to_send: usize = std::cmp::min(std::u32::MAX as u64, remaining) as usize;
            let sent = libc::sendfile(dest.my_raw_fd(), src.as_raw_fd(), offset, to_send as usize);

            if sent == 0 {
                return Err(CopyFailure::Errno(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "sendfile didn't want to send anything",
                )));
            }

            if sent < 0 {
                let error = io::Error::last_os_error();
                if let Some(code) = error.raw_os_error() {
                    if libc::EAGAIN == code {
                        continue;
                    }

                    if len == remaining && (libc::EINVAL == code || libc::ENOSYS == code) {
                        return Err(CopyFailure::Unsupported);
                    }

                }

                return Err(CopyFailure::Errno(error));
            }
            remaining -= sent as u64;
        }
    }
    Ok(())
}

fn try_streams(src: &mut File, dest: &mut io::Write, len: u64) -> Result<(), ()> {
    assert_eq!(len, std::io::copy(src, dest).unwrap());
    Ok(())
}

pub fn copy_file<T: MyRawFd + io::Write>(
    src: &mut File,
    dest: &mut T,
    len: u64,
) -> Result<(), io::Error> {
    // TODO: copy_file_range

    match try_sendfile(src, dest, len) {
        Ok(()) => return Ok(()),
        Err(fail) => {
            match fail {
                CopyFailure::Errno(x) => return Err(x),
                CopyFailure::Unsupported => (),
            }
        }
    };

    try_streams(src, dest, len).unwrap();

    Ok(())
}
