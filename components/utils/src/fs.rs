use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Write;

use std::path::Path;

pub fn read_bytes<P: AsRef<Path>>(input: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(input)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

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
    if let Some(parent) = o.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let result = std::fs::copy(i, o);
    // Discard the number of bytes copied
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
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

pub fn write_string<P: AsRef<Path>, S: AsRef<str>>(
    output: P,
    content: S,
) -> io::Result<()> {
    write_all(output, content.as_ref().as_bytes())
}
