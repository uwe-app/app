use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::io::Write;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;

use log::{debug};

pub fn read_string<P: AsRef<Path>>(input: P) -> io::Result<String> {
    let file = File::open(input)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    Ok(contents) 
}

pub fn copy<P: AsRef<Path>>(input: P, output: P) -> io::Result<()> {
    let i = input.as_ref();
    let o = output.as_ref();
    debug!("COPY {} -> {}", i.display(), o.display());
    if let Some(parent) = o.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let result = std::fs::copy(i, o);
    // Discard the number of bytes copied
    match result {
        Ok(_) => {
            Ok(()) 
        },
        Err(e) => Err(e)
    }
}

pub fn write_all<P: AsRef<Path>>(output: P, content: &[u8]) -> io::Result<()> {
    let o = output.as_ref();
    if let Some(parent) = o.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(o)?;
    file.write_all(content)
}

pub fn write_string<P: AsRef<Path>>(output: P, content: String) -> io::Result<()> {
    write_all(output, content.as_bytes())
}

