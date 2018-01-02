use std::path::Path;
use std::path::PathBuf;

use tempfile_fast::PersistableTempFile;

use errors::*;

pub struct ShardedStore {
    outdir: PathBuf,
}

impl ShardedStore {
    pub fn new<P: AsRef<Path>>(outdir: P) -> ShardedStore {
        ShardedStore {
            outdir: outdir.as_ref().to_path_buf(),
        }
    }

    pub fn store(&self, file: &PersistableTempFile, hash: &[u8]) -> Result<i64> {
        unimplemented!()
    }

    pub fn locality(&self) -> &Path {
        self.outdir.as_path()
    }
}
