use std::fs::{remove_file, File};
use std::path::{Path, PathBuf};

use sha3::{Digest, Sha3_256};
use tar::Archive;
use xz2::write::XzDecoder;

use scopeguard::defer;

use crate::{reader::read_path, Error, Result};

use config::{Plugin, PLUGIN};

#[derive(Default)]
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

    /// Whether to overwrite files when unpacking.
    overwrite: bool,

    /// Whether to peek into the bundled pugin.toml
    /// file and skip unpacking.
    peek: bool,
}

impl PackageReader {
    pub fn new(source: PathBuf) -> Self {
        Self {
            source,
            target: PathBuf::new(),
            expects: None,
            digest: Vec::new(),
            tarball: None,
            plugin: None,
            overwrite: false,
            peek: false,
        }
    }

    /// Configure the destination target directory for extraction.
    pub fn destination<D: AsRef<Path>>(mut self, dest: D) -> Result<Self> {
        if !self.source.exists() || !self.source.is_file() {
            return Err(Error::PackageSourceNotFile(self.source));
        }

        self.target = dest.as_ref().to_path_buf();

        Ok(self)
    }

    pub fn set_expects_checksum(mut self, expects: Option<Vec<u8>>) -> Self {
        self.expects = expects;
        self
    }

    pub fn set_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    pub fn set_peek(mut self, peek: bool) -> Self {
        self.peek = peek;
        self
    }

    /// Compute the SHA3-256 checksum for the compressed archive.
    pub async fn digest(mut self) -> Result<Self> {
        let mut reader = File::open(&self.source)?;
        let mut hasher = Sha3_256::new();
        std::io::copy(&mut reader, &mut hasher)?;

        self.digest = hasher.finalize().as_slice().to_owned();

        if let Some(ref expected) = self.expects {
            if expected != &self.digest {
                log::debug!("Expected {}", hex::encode(expected));
                log::debug!("Received {}", hex::encode(self.digest));
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
        let plugin = read_path(plugin_temp_file).await?;

        drop(archive);
        drop(tarball);

        if self.peek {
            self.plugin = Some(plugin);
            self.target = tarball_path.to_path_buf();
            return Ok(self);
        }

        // Determine the target extraction path
        /*
        let target = if let Some(ref builder) = self.path_builder {
            builder(&self.target, &plugin, &self.digest)?
        } else {
            self.target.clone()
        };
        */

        let target = self.target.clone();

        if target.exists() && !self.overwrite {
            return Err(Error::PackageOverwrite(
                plugin.name().to_string(),
                plugin.version().to_string(),
                target,
            ));
        }

        // Now unpack Vthe entire archive
        let mut tarball = File::open(&tarball_path)?;
        let mut archive = Archive::new(&mut tarball);
        archive.set_overwrite(self.overwrite);
        archive.unpack(&target)?;

        self.plugin = Some(plugin);
        self.target = target;

        Ok(self)
    }

    /// Retrieve the final destination path and digest.
    pub fn into_inner(self) -> (PathBuf, Vec<u8>, Plugin) {
        (self.target, self.digest, self.plugin.unwrap())
    }
}
