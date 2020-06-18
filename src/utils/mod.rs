use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Write;

use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use super::asset::Asset;

use inflector::Inflector;

use pulldown_cmark::{html, Options as MarkdownOptions, Parser};

use super::{BuildOptions, Error, INDEX_STEM};

use crate::build::page::Page;

use log::{debug};

pub mod git;
pub mod url;

pub fn generate_id(len: i32) -> String {
    let mut s: String = "".to_owned();
    for _ in 0..len {
        let x = rand::random::<u8>();
        s.push_str(&format!("{:x}", x));
    }
    s
}

pub fn is_draft(data: &Page, opts: &BuildOptions) -> bool {
    if opts.release {
        return data.draft.is_some() && data.draft.unwrap();
    }
    false
}

pub fn zip_from_file<P: AsRef<Path>>(archive: P, file: P, prefix: P) -> zip::result::ZipResult<()> {

    if let Ok(rel) = file.as_ref().strip_prefix(prefix) {
        let w = File::create(archive)?;
        let mut zip = zip::ZipWriter::new(w);
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        let bytes = read_bytes(file.as_ref())?;
        let rel_name = rel.to_string_lossy().into_owned();
        zip.start_file(rel_name, options)?;
        zip.write(&bytes)?;
        zip.finish()?;
    }

    Ok(())
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

pub fn write_string<P: AsRef<Path>>(output: P, content: String) -> io::Result<()> {
    write_all(output, content.as_bytes())
}

pub fn copy_asset_bundle_file(f: &str, template_name: &str, output: &PathBuf) -> Result<PathBuf, Error> {
    let mut s = template_name.clone().to_string();
    if !template_name.is_empty() {
        s.push('/');
    }
    s.push_str(f);

    let mut out = output.clone();
    out.push(f);
    debug!("copy {} -> {}", s, out.display());
    let dir = Asset::get(&s);
    match dir {
        Some(f) => {
            write_all(&out, &f)?;
        },
        None  => return Err(
            Error::new("Application bundle source file not found".to_string()))
    }
    Ok(out)
}


//pub fn write_string_minify<P: AsRef<Path>>(output: P, content: String) -> io::Result<()> {
    //let o = output.as_ref();
    //if let Some(parent) = o.parent() {
        //std::fs::create_dir_all(parent)?;
    //}

    //let mut file = File::create(o)?;
    //minify::minify(&mut content.as_bytes(), &mut file)

    ////write_all(output, content.as_bytes())
//}

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
