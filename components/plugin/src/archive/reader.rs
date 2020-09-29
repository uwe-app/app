use std::fs::{remove_file, File};
use std::path::{Path, PathBuf};

use sha3::{Digest, Sha3_256};
use tar::Archive;
use xz2::write::XzDecoder;

use log::debug;
use scopeguard::defer;

use crate::{reader::read_path, Error, Result};

use config::{Plugin, PLUGIN};

#[derive(Debug, Default)]
pub struct PackageReader {
    source: PathBuf,
    target: PathBuf,

    /// An expected checksum the archive should match.
    expects: Option<Vec<u8>>,

    /// Computed digest for the package.
    digest: Vec<u8>,

    /// Intermediary temp file used between decompression and extraction.
    tarball: Option<PathBuf>,

    /// The plugin information.
    plugin: Option<Plugin>,
}

impl PackageReader {
    pub fn new(source: PathBuf, expects: Option<Vec<u8>>) -> Self {
        Self {
            source,
            target: PathBuf::new(),
            expects,
            digest: Vec::new(),
            tarball: None,
            plugin: None,
        }
    }

    /// Configure the destination target directory for extraction.
    pub fn destination<D: AsRef<Path>>(mut self, dest: D) -> Result<Self> {
        if !self.source.exists() || !self.source.is_file() {
            return Err(Error::PackageSourceNotFile(self.source));
        }

        if !dest.as_ref().is_dir() {
            return Err(Error::PackageTargetNotDirectory(
                dest.as_ref().to_path_buf(),
            ));
        }

        self.target = dest.as_ref().to_path_buf();

        Ok(self)
    }

    /// Compute the SHA3-256 checksum for the compressed archive.
    pub async fn digest(mut self) -> Result<Self> {
        let mut reader = File::open(&self.source)?;
        let mut hasher = Sha3_256::new();
        std::io::copy(&mut reader, &mut hasher)?;

        self.digest = hasher.finalize().as_slice().to_owned();

        if let Some(ref expected) = self.expects {
            if expected != &self.digest {
                debug!("Expected {}", hex::encode(expected));
                debug!("Received {}", hex::encode(self.digest));
                return Err(Error::DigestMismatch(self.source));
            }
        }

        Ok(self)
    }

    /// Decompress the archive.
    pub async fn xz(mut self) -> Result<Self> {
        let mut reader = File::open(&self.source)?;
        let tarball = tempfile::NamedTempFile::new()?;
        let mut decoder = XzDecoder::new(tarball.as_file());
        std::io::copy(&mut reader, &mut decoder)?;
        decoder.finish()?;
        drop(decoder);

        // NOTE: must keep the temp file otherwise it is dropped
        // NOTE: when this function returns
        self.tarball = Some(tarball.into_temp_path().keep()?);

        Ok(self)
    }

    /// Unpack the contents of the archive.
    pub async fn tar(mut self) -> Result<Self> {
        let tarball_path = self.tarball.as_ref().unwrap().clone();

        defer! {
            let _ = remove_file(&tarball_path);
        }

        let mut tarball = File::open(&tarball_path)?;

        //let metadata = tarball.metadata()?;
        //println!("Meta {:#?}", metadata.len());
        //println!("Unpacking archive...");

        let mut plugin_path: Option<PathBuf> = None;

        // Extract the plugin.toml file first if we can
        let mut archive = Archive::new(&mut tarball);
        for entry in archive.entries()? {
            let mut file = entry?;
            let path = file.path()?;
            let name = path.to_string_lossy().into_owned();
            if name == PLUGIN {
                let plugin_temp = tempfile::NamedTempFile::new()?;
                let plugin_temp_path = plugin_temp.into_temp_path().keep()?;
                file.unpack(&plugin_temp_path)?;
                plugin_path = Some(plugin_temp_path);
                break;
            }
        }

        if plugin_path.is_none() {
            return Err(Error::InvalidArchiveNoPluginFile(
                self.source,
                PLUGIN.to_string(),
            ));
        }

        let plugin_temp_file = plugin_path.as_ref().unwrap();

        defer! {
            let _ = remove_file(&plugin_temp_file);
        }

        // Read in the plugin data before unpacking the entire archive
        self.plugin = Some(read_path(plugin_temp_file).await?);

        // Now unpack the entire archive
        drop(archive);
        drop(tarball);

        let mut tarball = File::open(&tarball_path)?;
        let mut archive = Archive::new(&mut tarball);
        archive.unpack(&self.target)?;

        Ok(self)
    }

    /// Retrieve the final destination path and digest.
    pub fn into_inner(self) -> (PathBuf, Vec<u8>, Plugin) {
        (self.target, self.digest, self.plugin.unwrap())
    }
}