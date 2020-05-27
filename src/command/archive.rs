use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use crate::Error;

use ignore::WalkBuilder;
use crate::utils;

use log::{info, warn};

#[derive(Debug)]
pub struct ArchiveOptions {
    pub source: PathBuf,
    pub target: Option<PathBuf>,
    pub force: bool,
}

pub fn archive(options: ArchiveOptions) -> Result<(), Error> {
    let mut dest = Path::new("").to_path_buf();
    if let Some(target) = options.target {
        dest = target;
    } else {
        if let Some(name) = options.source.file_name() {
            dest = Path::new(name).to_path_buf(); 
        } 
    }

    if dest == Path::new("").to_path_buf() {
        return Err(Error::new("failed to determine archive target".to_string()))
    }

    dest.set_extension("zip");

    if dest.exists() {
        if options.force {
            info!("rm {}", dest.display());
            fs::remove_file(&dest)?;
        } else {
            return Err(
                Error::new(
                    format!(
                        "{} already exists, use --force to overwrite", dest.display())))
        }
    }

    info!("{}", dest.display());

    let w = File::create(&dest)?;
    let mut zip = zip::ZipWriter::new(w);
    let opts = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    for result in WalkBuilder::new(&options.source).build() {
        match result {
            Ok(entry) => {
                let path = entry.path();

                if let Ok(rel) = path.strip_prefix(&options.source) {

                    // Skip root directory
                    if rel == Path::new("") {
                        continue; 
                    }

                    if path.is_file() {
                        info!("add {}", rel.display());

                        let bytes = utils::read_bytes(path)?;
                        let rel_name = rel.to_string_lossy().into_owned();
                        zip.start_file(rel_name, opts)?;
                        zip.write(&bytes)?;
                    }

                } else {
                    warn!("failed to get relative path for {}", path.display());
                }
            },
            Err(e) => return Err(Error::from(e))
        }
    }

    zip.finish()?;

    Ok(())
}
