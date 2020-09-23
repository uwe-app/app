use std::path::{Path, PathBuf};

use std::io::prelude::*;
use std::fs::File;
use tar::Builder;
use xz2::{write::XzEncoder, stream::Check, stream::Stream};

use log::debug;

use config::PLUGIN;

use crate::{Error, Result, walk};

#[derive(Debug, Default)]
pub struct PackageWriter {
    source: PathBuf,
    target: PathBuf,
}

impl PackageWriter {

    pub fn new(source: PathBuf) -> Self {
        let source = if source.ends_with(PLUGIN) {
            source.parent().unwrap().to_path_buf()
        } else { source };

        Self { source, target: PathBuf::new() }
    }

    pub fn destination<D: AsRef<Path>>(mut self, dest: D, relative: bool) -> Result<Self> {
        // Make the destination relative to the source
        self.target = if relative {
            self.source.join(dest.as_ref())
        } else {
            dest.as_ref().to_path_buf()
        };
        Ok(self)
    }

    pub async fn tar(mut self) -> Result<Self> {
        let src = &self.source;
        self.target.set_extension("tar");

        if self.target.exists() {
            return Err(Error::TarPackageExists(self.target.clone()))
        }

        debug!("Create tar archive {}", self.target.display());

        let file = File::create(&self.target)?;
        let mut tarball = Builder::new(file);

        let mut files = walk::find(src, |_| true);
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

    pub async fn xz(mut self) -> Result<Self> {
        let source = self.target.clone();

        //println!("Compress with source file {}", source.display());

        self.target.set_extension("tar.xz");

        if self.target.exists() {
            return Err(Error::TarPackageExists(self.target.clone()))
        }

        //println!("Compress with target file {}", self.target.display());

        let stream = Stream::new_easy_encoder(9, Check::Crc64)?;
        let mut reader = File::open(&source)?;
        let mut writer = File::create(&self.target)?;
        let mut encoder = XzEncoder::new_stream(writer, stream);

        std::io::copy(&mut reader, &mut encoder);

        encoder.try_finish()?;

        Ok(self)
    }

    pub async fn checksum(mut self) -> Result<Self> {
        todo!();
        Ok(self)
    }

    pub fn into_inner() {

    }
}
