use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Write;

use std::path::Path;
use std::path::PathBuf;

use inflector::Inflector;

use pulldown_cmark::{html, Options as MarkdownOptions, Parser};

use crate::{Result, Error};
use crate::build::page::Page;
use super::{INDEX_STEM};

use crate::build::CompilerOptions;

use log::{info, debug};

pub mod merge;
pub mod symlink;
pub mod url;

pub fn generate_id(len: i32) -> String {
    let mut s: String = "".to_owned();
    for _ in 0..len {
        let x = rand::random::<u8>();
        s.push_str(&format!("{:x}", x));
    }
    s
}

pub fn require_output_dir(output: &PathBuf) -> Result<()> {
    if !output.exists() {
        info!("mkdir {}", output.display());
        fs::create_dir_all(output)?;
    }

    if !output.is_dir() {
        return Err(
            Error::new(
                format!("Not a directory: {}", output.display())));
    }

    Ok(())
}

pub fn is_draft(data: &Page, opts: &CompilerOptions) -> bool {
    if opts.release {
        return data.draft.is_some() && data.draft.unwrap();
    }
    false
}

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
    debug!("copy {} -> {}", i.display(), o.display());
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

pub fn write_string<P: AsRef<Path>, S: AsRef<str>>(output: P, content: S) -> io::Result<()> {
    write_all(output, content.as_ref().as_bytes())
}

// Convert a file name to title case
pub fn file_auto_title<P: AsRef<Path>>(input: P) -> Option<String> {
    let i = input.as_ref();
    if let Some(nm) = i.file_stem() {
        // If the file is an index file, try to get the name
        // from a parent directory
        if nm == INDEX_STEM {
            if let Some(p) = i.parent() {
                return file_auto_title(&p.to_path_buf());
            }
        } else {
            let auto = nm.to_str().unwrap().to_string();
            let capitalized = auto.to_title_case();
            return Some(capitalized);
        }
    }
    None
}

pub fn render_markdown_string(content: &str) -> String {
    let mut options = MarkdownOptions::empty();
    options.insert(MarkdownOptions::ENABLE_TABLES);
    options.insert(MarkdownOptions::ENABLE_FOOTNOTES);
    options.insert(MarkdownOptions::ENABLE_STRIKETHROUGH);
    options.insert(MarkdownOptions::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(content, options);
    let mut markup = String::new();
    html::push_html(&mut markup, parser);
    markup
}
