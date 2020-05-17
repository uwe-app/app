use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::io::Write;
use std::fs::File;
use std::path::PathBuf;

use log::{debug};

pub fn read_string(input: &PathBuf) -> io::Result<String> {
    let file = File::open(input)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    Ok(contents) 
}

pub fn copy(input: PathBuf, output: PathBuf) -> io::Result<()> {
    debug!("COPY {} -> {}", input.display(), output.display());
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let result = std::fs::copy(input, output);
    // Discard the number of bytes copied
    match result {
        Ok(_) => {
            Ok(()) 
        },
        Err(e) => Err(e)
    }
}

pub fn write_all(output: PathBuf, content: &[u8]) -> io::Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(output)?;
    file.write_all(content)
}

pub fn write_string(output: PathBuf, content: String) -> io::Result<()> {
    write_all(output, content.as_bytes())
}

