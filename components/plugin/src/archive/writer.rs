use std::fs::{remove_file, File};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sha3::{Digest, Sha3_256};
use tar::{Builder, EntryType, Header};
use xz2::{stream::Check, stream::Stream, write::XzEncoder};

use log::debug;

use config::PLUGIN;
use utils::walk;

use crate::{reader::normalize, Error, Result};

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
        } else {
            source
        };

        Self {
            source,
            target: PathBuf::new(),
            digest: Vec::new(),
        }
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

        let mut xz_file = self.target.clone();
        xz_file.set_extension("tar.xz");

        if self.target.exists() {
            return Err(Error::PackageExists(self.target.clone()));
        } else if xz_file.exists() {
            return Err(Error::PackageExists(xz_file));
        }

        debug!("Create tar archive {}", self.target.display());

        let plugin_path = Path::new(PLUGIN);
        let mut plugin_original = PathBuf::from(plugin_path);
        plugin_original.set_extension("orig.toml");

        let file = File::create(&self.target)?;
        let mut tarball = Builder::new(file);

        let files = walk::find(src, |_| true);
        for file in files.into_iter() {
            if file.is_file() {
                // Protect against recursively adding the package.tar file
                if file.canonicalize()? == self.target.canonicalize()? {
                    continue;
                }

                let rel = file.strip_prefix(src)?;
                if rel == plugin_path {
                    debug!(
                        "Create normalized plugin file {}",
                        plugin_path.display()
                    );

                    let (original, plugin) = normalize(&file, true).await?;
                    append_file(
                        &mut tarball,
                        &plugin_original,
                        original.as_bytes(),
                    )?;
                    append_file(&mut tarball, &plugin_path, plugin.as_bytes())?;
                } else {
                    tarball.append_file(rel, &mut File::open(&file)?)?;
                }
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
            return Err(Error::PackageExists(self.target.clone()));
        }

        let stream = Stream::new_easy_encoder(9, Check::Crc64)?;
        let mut reader = File::open(&source)?;

        let mut encoder =
            XzEncoder::new_stream(File::create(&self.target)?, stream);

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

/// Helper to append a byte slice to a tarball archive.
fn append_file<P: AsRef<Path>>(
    tarball: &mut Builder<File>,
    path: P,
    contents: &[u8],
) -> Result<()> {
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::file());
    header.set_mode(0o644);
    header.set_mtime(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    );
    header.set_size(contents.len() as u64);
    header.set_cksum();
    tarball.append_data(&mut header, path.as_ref(), contents)?;
    Ok(())
}
