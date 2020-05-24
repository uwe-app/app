use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug)]
pub enum FileType {
    Markdown,
    Html,
    Private,
    Unknown,
}

use crate::{
    Error,
    HTML,
    INDEX_STEM,
    LAYOUT_HBS,
    LAYOUT_TOML,
    MD,
    PARSE_EXTENSIONS,
    TEMPLATE,
    THEME,
    TOML,
};

pub fn get_theme_dir<P: AsRef<Path>>(base: P) -> PathBuf {
    let mut root_theme = base.as_ref().to_path_buf();
    root_theme.push(TEMPLATE);
    root_theme.push(THEME);
    root_theme
}

pub fn is_index<P: AsRef<Path>>(file: P) -> bool {
    if let Some(nm) = file.as_ref().file_stem() {
        if nm == INDEX_STEM {
            return true;
        }
    }
    false
}

pub fn get_type<P: AsRef<Path>>(p: P) -> FileType {
    let file = p.as_ref();
    match file.file_name() {
        Some(nm) => {
            if let Some(nm) = nm.to_str() {
                if nm == LAYOUT_HBS || nm == LAYOUT_TOML {
                    return FileType::Private;
                } else {
                    if let Some(ext) = file.extension() {
                        if ext == MD {
                            return FileType::Markdown;
                        } else if ext == HTML {
                            return FileType::Html;
                        } else if ext == TOML && has_parse_file_match(file) {
                            return FileType::Private;
                        }
                    }
                }
            }
        }
        _ => {}
    }
    FileType::Unknown
}

pub fn has_parse_file_match<P: AsRef<Path>>(file: P) -> bool {
    let path = file.as_ref();
    let mut copy = path.to_path_buf();
    for ext in PARSE_EXTENSIONS.iter() {
        copy.set_extension(ext);
        if copy.exists() {
            return true;
        }
    }
    false
}

pub fn clean<P: AsRef<Path>>(file: P, result: P) -> Option<PathBuf> {
    let clean_target = file.as_ref().clone();
    if !is_index(&clean_target) {
        if let Some(parent) = clean_target.parent() {
            if let Some(stem) = clean_target.file_stem() {
                let mut target = parent.to_path_buf();
                target.push(stem);
                target.push(INDEX_STEM);

                if !has_parse_file_match(&target) {
                    let clean_result = result.as_ref().clone();
                    if let Some(parent) = clean_result.parent() {
                        if let Some(stem) = clean_result.file_stem() {
                            let mut res = parent.to_path_buf();
                            res.push(stem);
                            res.push(INDEX_STEM);
                            res.set_extension(HTML);
                            return Some(res);
                        }
                    }
                }
            }
        }
    }
    None
}

// Build the destination file path.
pub fn destination<P: AsRef<Path>>(
    source: P,
    target: P,
    file: P,
    file_type: &FileType,
    clean_urls: bool,
) -> Result<PathBuf, Error> {
    let pth = file.as_ref();
    let relative = pth.strip_prefix(source.as_ref());

    println!("matcher replacing {}", source.as_ref().display());
    println!("matcher relative {:?}", relative);

    match relative {
        Ok(relative) => {
            let mut result = target.as_ref().clone().join(relative);
            match file_type {
                FileType::Markdown | FileType::Html => {
                    result.set_extension(HTML);
                    if clean_urls {
                        if let Some(res) = clean(pth, &result) {
                            result = res;
                        }
                    }
                }
                _ => {}
            }
            return Ok(result)
        }
        Err(e) => return Err(Error::from(e))
    }
}
