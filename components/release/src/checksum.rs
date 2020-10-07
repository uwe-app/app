use std::fs::File;
use std::path::Path;

use sha3::{Digest, Sha3_256};

use crate::Result;

/// Compute the SHA3-256 checksum for a file.
pub(crate) fn digest<P: AsRef<Path>>(target: P) -> Result<Vec<u8>> {
    let mut reader = File::open(target.as_ref())?;
    let mut hasher = Sha3_256::new();
    std::io::copy(&mut reader, &mut hasher)?;
    Ok(hasher.finalize().as_slice().to_owned())
}

