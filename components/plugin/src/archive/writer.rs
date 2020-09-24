use std::path::{Path, PathBuf};
use std::fs::{File, remove_file};

use tar::Builder;
use xz2::{write::XzEncoder, stream::Check, stream::Stream};
use sha3::{Digest, Sha3_256};

use log::debug;

use config::PLUGIN;

use crate::{Error, Result, walk};

#[derive(Debug, Default)]
pub struct PackageWriter {
    source: PathBuf,
    target: PathBuf,
    digest: Vec<u8>,
}

impl PackageWriter {

    pub fn new(source: PathBuf) -> Self {
        let source = if source.ends_with(PLUGIN) {
            source.parent().unwrap().to_path_buf()
        } else { source };

        Self { source, target: PathBuf::new(), digest: Vec::new() }
    }

    /// Configure the destination target file.
    pub fn destination<D: AsRef<Path>>(mut self, dest: D) -> Result<Self> {
        self.target = dest.as_ref().to_path_buf();
        Ok(self)
    }

    /// Create a tar archive from the source plugin.
    pub async fn tar(mut self) -> Result<Self> {
        let src = &self.source;
        self.target.set_extension("tar");

        if self.target.exists() {
            return Err(Error::PackageExists(self.target.clone()))
        }

        debug!("Create tar archive {}", self.target.display());

        let file = File::create(&self.target)?;
        let mut tarball = Builder::new(file);

        let files = walk::find(src, |_| true);
        for file in files.into_iter() {
            if file.is_file() {
                let rel = file.strip_prefix(src)?;
                //println!("Got tarball file {} {}", rel.display(), file.display());
                tarball.append_file(rel, &mut File::open(&file)?)?;
            }
        }

        tarball.into_inner()?;

        Ok(self)
    }

    /// Compress the tarball with lzma and update the target file extension.
    pub async fn xz(mut self) -> Result<Self> {
        let source = self.target.clone();

        self.target.set_extension("tar.xz");

        if self.target.exists() {
            return Err(Error::PackageExists(self.target.clone()))
        }

        let stream = Stream::new_easy_encoder(9, Check::Crc64)?;
        let mut reader = File::open(&source)?;

        let mut encoder = XzEncoder::new_stream(File::create(&self.target)?, stream);

        std::io::copy(&mut reader, &mut encoder)?;

        encoder.try_finish()?;
        remove_file(&source)?;

        Ok(self)
    }

    /// Compute the SHA3-256 checksum for the compressed archive.
    pub async fn digest(mut self) -> Result<Self> {
        let mut reader = File::open(&self.target)?;
        let mut hasher = Sha3_256::new();
        std::io::copy(&mut reader, &mut hasher)?;

        self.digest = hasher.finalize().as_slice().to_owned();
        Ok(self)
    }

    /// Retrieve the final destination path and digest.
    pub fn into_inner(self) -> (PathBuf, Vec<u8>) {
        (self.target, self.digest)
    }
}
