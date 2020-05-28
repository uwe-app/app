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
    DATA_TOML,
    MD,
    PARSE_EXTENSIONS,
    TEMPLATE,
    THEME,
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

pub fn collides<P: AsRef<Path>>(file: P, file_type: &FileType) -> (bool, PathBuf) {
    let mut other = file.as_ref().to_path_buf(); 
    match file_type {
        FileType::Markdown => {
            other.set_extension(HTML);
            return (other.exists(), other)
        },
        FileType::Html => {
            other.set_extension(MD);
            return (other.exists(), other)
        }
        _ => return (false, Path::new("").to_path_buf())
    }
}

pub fn get_type<P: AsRef<Path>>(p: P) -> FileType {
    let file = p.as_ref();
    match file.file_name() {
        Some(nm) => {
            if let Some(nm) = nm.to_str() {
                if nm == LAYOUT_HBS || nm == DATA_TOML {
                    return FileType::Private
                } else {
                    if let Some(ext) = file.extension() {
                        if ext == MD {
                            return FileType::Markdown
                        } else if ext == HTML {
                            return FileType::Html
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

pub fn is_clean<P: AsRef<Path>>(file: P) -> bool {
    let target = file.as_ref().to_path_buf();
    let result = target.clone();
    return clean(target, result).is_some();
}

pub fn clean<P: AsRef<Path>>(file: P, result: P) -> Option<PathBuf> {
    let clean_target = file.as_ref();
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

    // NOTE: When watching files we can get absolute
    // NOTE: paths passed for `file` even when `source`
    // NOTE: is relative. This handles that case by making
    // NOTE: the `source` absolute based on the current working
    // NOTE: directory.
    let mut src: PathBuf = source.as_ref().to_path_buf();
    if pth.is_absolute() && src.is_relative() {
        if let Ok(cwd) = std::env::current_dir() {
            src = cwd.clone();
            src.push(source.as_ref())
        }
    }

    let relative = pth.strip_prefix(src);

    //println!("matcher replacing {}", source.as_ref().display());
    //println!("matcher relative {:?}", relative);

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
