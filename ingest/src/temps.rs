use std::fs;
use std::io;
use std::path::Path;
use std::io::Read;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;

use base32;
use ci_capnp;
use thread_pool;
use lz4;
use num_cpus;
use sha2;
use tempfile_fast;
use sha2::Digest;

use errors::*;


fn tools() -> (sha2::Sha256, lz4::EncoderBuilder) {
    (sha2::Sha256::default(), lz4::EncoderBuilder::new())
}

fn is_text(buf: &[u8]) -> bool {
    for char in buf {
        if 0 == *char // NUL
            // ENQ (enquiry), ACK (acknowledge),
            // \a (bell) and \b (backspace)
            || (*char >= 5 && *char <= 8)
            // SO, SI, DLE, DC?, NAK, SYN, ETB, CAN, EM, SUB, ESC (colour codes?),
            // FS, GS, RS, US
            || (*char >= 14 && *char < 32)
        {
            return false;
        }
    }

    true
}

fn hash_compress_write_from_slice<W>(buf: &[u8], to: W) -> ([u8; 256 / 8], bool)
where
    W: Write,
{
    let (mut hasher, lz4) = tools();
    let mut lz4 = lz4.build(to).expect("lz4 writer");

    hasher.input(buf);
    lz4.write_all(buf).expect("lz4 writing");
    lz4.finish();

    (to_bytes(&hasher.result()[..]), is_text(buf))
}

fn hash_compress_write_from_reader<R, W>(mut from: R, to: W) -> (u64, [u8; 256 / 8], bool)
where
    W: Write,
    R: Read,
{
    let (mut hasher, lz4) = tools();
    let mut lz4 = lz4.build(to).expect("lz4 writer");
    let mut text = true;

    let mut total_read = 0u64;
    loop {
        let mut buf = [0u8; 4096 * 16];

        let read = from.read(&mut buf).expect("reading");
        if 0 == read {
            break;
        }

        total_read += read as u64;

        hasher.input(&buf[0..read]);
        lz4.write_all(&buf[0..read]).expect("lz4 written");

        if text {
            text &= is_text(&buf[0..read]);
        }
    }
    let (_, result) = lz4.finish();
    result.expect("lz4 finished");

    (total_read, to_bytes(&hasher.result()[..]), text)
}

fn to_bytes(slice: &[u8]) -> [u8; 256 / 8] {
    let mut hash = [0u8; 256 / 8];
    hash.clone_from_slice(slice);
    hash
}

#[derive(Debug)]
pub struct TempFile {
    pub header: ci_capnp::FileEntry,
    pub packed_len: u64,
    pub hash: [u8; 256 / 8],
    pub text: bool,
    pub name: String,
}

pub fn read(out_dir: &str) -> Result<Vec<TempFile>> {
    if !Path::new(format!("{}/zz", out_dir).as_str()).is_dir() {
        let alphabet_chars = "234567abcdefghijklmnopqrstuvwxyz";
        for first in alphabet_chars.chars() {
            for second in alphabet_chars.chars() {
                fs::create_dir_all(format!("{}/{}{}", out_dir, first, second))?;
            }
        }
    }

    let store: Vec<TempFile> = vec![];
    let dest = Arc::new(Mutex::new(store));

    let (sender, pool) = thread_pool::Builder::new()
        .work_queue_capacity(num_cpus::get() * 2)
        .build();

    let mut pool_used = false;

    let mut stdin = io::stdin();
    while let Some(en) = ci_capnp::read_entry(&mut stdin).expect("capnp") {
        if 0 == en.len {
            continue;
        }

        let mut temp = tempfile_fast::PersistableTempFile::new_in(&out_dir)?;

        if en.len < 16 * 1024 * 1024 {
            let mut buf = vec![0u8; en.len as usize];
            stdin.read_exact(&mut buf).expect("read");

            let out_dir = out_dir.to_string();
            let dest = dest.clone();
            sender
                .send(move || {
                    let (hash, text) = hash_compress_write_from_slice(&buf, temp.as_mut());

                    complete(en, temp, hash, out_dir.as_str(), text, &dest).unwrap();
                })
                .expect("offloading");
            pool_used = true;
        } else {
            let file_data = (&mut stdin).take(en.len);
            let (total_read, hash, text) =
                hash_compress_write_from_reader(file_data, temp.as_mut());
            assert_eq!(en.len, total_read);

            complete(en, temp, hash, out_dir, text, &dest)?;
        }
    }

    pool.shutdown();

    // This deadlocks if the pool has never been used: https://github.com/carllerche/thread-pool/issues/5
    if pool_used {
        pool.await_termination();
    }

    Ok(Arc::try_unwrap(dest).unwrap().into_inner().unwrap())
}

fn complete(
    en: ci_capnp::FileEntry,
    temp: tempfile_fast::PersistableTempFile,
    hash: [u8; 256 / 8],
    out_dir: &str,
    text: bool,
    store: &Mutex<Vec<TempFile>>,
) -> Result<()> {
    let encoded_hash = encode_hash(&hash);
    let written_to = format!(
        "{}/{}/1-{}.lz4",
        out_dir,
        &encoded_hash[0..2],
        &encoded_hash[2..]
    );
    let len = temp.metadata()?.len();

    {
        let written_to_path = Path::new(&written_to);

        if !written_to_path.exists() {
            if let Err(e) = temp.persist_noclobber(&written_to) {
                if !written_to_path.exists() {
                    bail!(e.error);
                }
            }
        }
    }

    store.lock().unwrap().push(TempFile {
        header: en,
        packed_len: len,
        name: written_to,
        hash,
        text,
    });
    Ok(())
}

pub fn encode_hash(hash: &[u8]) -> String {
    let mut encoded_hash = base32::encode(base32::Alphabet::RFC4648 { padding: false }, hash);
    encoded_hash.make_ascii_lowercase();
    encoded_hash
}
