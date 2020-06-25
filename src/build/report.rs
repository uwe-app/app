use std::collections::HashSet;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use md5::{Md5, Digest};

use crate::{Error, Result};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ResultFile {
    pub path: PathBuf,
    pub e_tag: String,
}

#[derive(Debug)]
pub struct FileBuilder {
    // A base path all files must be relative to
    pub base: PathBuf,
    // When specified this prefix is appended before the path
    pub prefix: Option<String>,
    // The list of collected paths when enabled
    pub paths: HashSet<ResultFile>,
}

impl FileBuilder {
    pub fn new(
        base: PathBuf,
        prefix: Option<String>) -> Self {
        Self {
            base,
            prefix,
            paths: HashSet::new(),
        }
    }

    // Compute a digest from the file on disc
    fn digest_path<P: AsRef<Path>>(&mut self, path: P) -> Result<String> {
        let mut file = std::fs::File::open(path)?;
        let chunk_size = 16_000;
        let mut hasher = Md5::new();
        loop {
            let mut chunk = Vec::with_capacity(chunk_size);
            let n = file.by_ref().take(chunk_size as u64).read_to_end(&mut chunk)?;
            hasher.update(chunk);
            if n == 0 || n < chunk_size { break; }
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn add<P: AsRef<Path>>(&mut self, raw: P) -> Result<()> {
        let e_tag = self.digest_path(&raw)?;

        let mut path = raw.as_ref().strip_prefix(&self.base)?.to_path_buf();
        path = if let Some(ref prefix) = self.prefix {
            let mut tmp = PathBuf::from(prefix);
            tmp.push(path);
            tmp
        } else {
            path
        };

        let result = ResultFile { path, e_tag };
        self.paths.insert(result);

        Ok(())
    }

    pub fn walk(&mut self) -> Result<()> {
        for result in WalkBuilder::new(&self.base)
            .follow_links(true)
            .build() {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        self.add(&path)?;
                    }
                }
                Err(e) => return Err(Error::from(e)),
            }
        }
        Ok(())
    }
}

