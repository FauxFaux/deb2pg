use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::path::Path;

use tempfile_fast::PersistableTempFile;
use tempfile_fast::persistable_tempfile_in;
use sha2::Digest;
use sha2::Sha512Trunc256;
use zstd;

use errors::*;

const DICT_LEN: usize = 112640;
type Dict = &'static [u8; DICT_LEN];
const DICT: [Dict; 8] = [
    include_bytes!("../../dicts/text.dictionary"), // 11
    include_bytes!("../../dicts/conf.dictionary"), // 12
    include_bytes!("../../dicts/c.dictionary"), // 13
    include_bytes!("../../dicts/oddlang.dictionary"), // 14
    include_bytes!("../../dicts/web.dictionary"), // 15

    include_bytes!("../../dicts/99.dictionary"), // 2
    include_bytes!("../../dicts/999.dictionary"), // 3
    include_bytes!("../../dicts/9999.dictionary"), // 4
];

enum CompressionType {
    Text,
    Conf,
    C,
    Code,
    Web,

    Tiny,
    Medium,
    Other,
}

pub struct TempFile {
    pub len: u64,
    pub hash: [u8; 256 / 8],
    pub file: PersistableTempFile,
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

fn dict_for(len: u64) -> Dict {
    if len < 100 {
        DICT[0]
    } else if len < 1000 {
        DICT[1]
    } else {
        DICT[2]
    }
}

pub fn hash_compress_write_from_reader<R: Read + Seek, P: AsRef<Path>>(
    mut from: R,
    inside: P,
) -> Result<Option<TempFile>> {
    let len = from.seek(SeekFrom::End(0))?;
    from.seek(SeekFrom::Start(0))?;

    let mut to = persistable_tempfile_in(inside)?;
    let mut hasher = Sha512Trunc256::default();
    let mut total_read = 0u64;

    {
        let mut compressor = zstd::Encoder::with_dictionary(to.as_mut(), 10, dict_for(len))?;

        loop {
            let mut buf = [0u8; 1024 * 8];

            let read = from.read(&mut buf)?;
            if 0 == read {
                break;
            }

            if !is_text(&buf[0..read]) {
                return Ok(None);
            }

            total_read += read as u64;

            hasher.input(&buf[0..read]);
            compressor.write_all(&buf[0..read])?;
        }
        compressor.finish()?;
    }

    Ok(Some(TempFile {
        len: total_read,
        hash: to_bytes(&hasher.result()[..]),
        file: to,
    }))
}

fn to_bytes(slice: &[u8]) -> [u8; 256 / 8] {
    let mut hash = [0u8; 256 / 8];
    hash.clone_from_slice(slice);
    hash
}


fn identify(path: &str) -> CompressionType {
    let text = [
        "control",
        "dsc",
        "doc",
        "rst",
        "txt",
        "po",
        "README",
        "LICENSE",
        "copyright",
        "md",
        "mo",
        "rdf",
        "qdoc",
        "manifest",
    ];

    let conf = [
        "Makefile",
        "properties",
        "ini",
        "in",
        "inc",
        "json",
        "am",
        "build",
        "gitignore",
        "sh",
        "m4",
        "mk",
        "pro",
        "rules",
        "cmake",
        "cfg",
        "conf",
        "gyp",
        "yaml",
        "qml",
    ];

    let c = [
        "hxx",
        "cpp",
        "c",
        "h",
        "hpp",
        "cc",
        "cxx",
        "d",
        "ml",
        "hh",
        "idl",
    ];

    let odd = [
        "cs",
        "java",
        "rb",
        "rs",
        "py",
        "go",
        "php",
        "phpt",
        "hs",
        "ll",
        "s",
        "pl",
        "pm",
    ];

    let web = [
        "xhtml",
        "dtd",
        "html",
        "js",
        "xml",
        "svg",
        "xht",
        "xul",
        "css",
        "htm",
        "sjs",
    ];

    unimplemented!()
}
