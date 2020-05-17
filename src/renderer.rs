use std::io;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use log::{debug};

pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Renderer{}
    }

    pub fn copy(&self, input: PathBuf, output: PathBuf) -> io::Result<()> {
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

    pub fn write_all(&mut self, output: PathBuf, content: &[u8]) -> io::Result<()> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = File::create(output)?;
        file.write_all(content)
    }

    pub fn write_string(&mut self, output: PathBuf, content: String) -> io::Result<()> {
        self.write_all(output, content.as_bytes())
    }
}
