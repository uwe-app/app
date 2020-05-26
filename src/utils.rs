use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::Write;

use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use super::asset::Asset;

use serde_json::{Map, Value};
use inflector::Inflector;

use pulldown_cmark::{html, Options as MarkdownOptions, Parser};

use super::{BuildOptions, Error, INDEX_STEM};
//use super::minify;

use log::{debug};

pub fn generate_id(len: i32) -> String {
    let mut s: String = "".to_owned();
    for _ in 0..len {
        let x = rand::random::<u8>();
        s.push_str(&format!("{:x}", x));
    }
    s
}

pub fn is_draft(data: &Map<String, Value>, opts: &BuildOptions) -> bool {
    if opts.release {
        if let Some(val) = data.get("draft") {
            return val.as_bool().is_some()
        }
    }
    false
}

/*
pub fn inherit<P: AsRef<Path>, S: AsRef<str>>(base: P, input: P, name: S) -> Option<PathBuf> {
    if let Some(p) = input.as_ref().parent() {
        for p in p.ancestors() {
            let mut copy = p.to_path_buf().clone();
            copy.push(name.as_ref());

            if copy.exists() {
                return Some(copy);
            }

            // Ensure we do not go beyond the base which coould happen
            // if the program source is an absolute path
            if base.as_ref() == &p.to_path_buf() {
                break;
            }
        }
    }
    None
}
*/

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
    s.push('/');
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
            Error::new("application bundle source file not found".to_string()))
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
