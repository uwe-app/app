use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::Index;
use crate::Result;

pub fn write<P: AsRef<Path>>(index: &Index, filename: P) -> Result<usize> {
    let file = File::create(filename)?;
    let mut bufwriter = BufWriter::new(file);
    let write_version = super::VERSION_STRING.as_bytes();
    write_release(index, &mut bufwriter, &write_version)
}

fn write_release(
    index: &Index,
    bufwriter: &mut BufWriter<File>,
    write_version: &[u8],
) -> Result<usize> {
    let mut bytes_written: usize = 0;
    let index_bytes = rmp_serde::to_vec(index)?;
    let byte_vectors_to_write = [write_version, index_bytes.as_slice()];
    for vec in byte_vectors_to_write.iter() {
        bytes_written += bufwriter.write(&(vec.len() as u64).to_be_bytes())?;
        bytes_written += bufwriter.write(vec)?;
    }
    Ok(bytes_written)
}
