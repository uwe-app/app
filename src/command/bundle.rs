use std::path::PathBuf;
use crate::Error;

#[derive(Debug)]
pub struct BundleOptions {
    pub target: PathBuf,
}

pub fn bundle(options: BundleOptions) -> Result<(), Error> {
    Ok(())
}
