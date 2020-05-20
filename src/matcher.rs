use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;

#[derive(Debug)]
pub enum FileType {
    Markdown,
    Html,
    Private,
    Unknown,
}

use super::{
    INDEX,
    THEME,
    MD,
    HTML,
    HBS,
    TOML,
    PARSE_EXTENSIONS,
};

pub fn get_theme_dir<P: AsRef<Path>>(base: P, template: &str) -> PathBuf {
    let mut root_theme = base.as_ref().to_path_buf();
    root_theme.push(template);
    root_theme.push(THEME);
    root_theme
}

pub fn is_index<P: AsRef<Path>>(file: P) -> bool {
    if let Some(nm) = file.as_ref().file_stem() {
        if nm == INDEX {
            return true
        } 
    } 
    false
}

pub fn get_type<P: AsRef<Path>>(layout: &str, file: P) -> FileType {

    let name = file.as_ref().file_name();
    let layout_toml = "layout.toml";
    match name {
        Some(nm) => {
            if let Some(nm) = nm.to_str() {
                if nm == layout || nm == layout_toml || nm.ends_with(HBS) {
                    return FileType::Private
                } else if nm.ends_with(MD) {
                    return FileType::Markdown
                } else if nm.ends_with(HTML) {
                    return FileType::Html
                } else if nm.ends_with(TOML) && has_parse_file_match(file) {
                    return FileType::Private
                }
            }
        },
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
                target.push(INDEX);

                if !has_parse_file_match(&target) {
                    let clean_result = result.as_ref().clone();
                    if let Some(parent) = clean_result.parent() {
                        if let Some(stem) = clean_result.file_stem() {
                            let mut res = parent.to_path_buf();
                            res.push(stem);
                            res.push(INDEX);
                            res.set_extension("html");
                            return Some(res)
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
    clean_urls: bool) -> PathBuf {

    let pth = file.as_ref();
    let relative = pth.strip_prefix(source.as_ref());
    match relative {
        Ok(relative) => {
            let mut result = target.as_ref().clone().join(relative);
            match file_type {
                FileType::Markdown | FileType::Html => {
                    result.set_extension("html");
                    if clean_urls {
                        if let Some(res) = clean(pth, &result) {
                            result = res;
                        }
                    }
                },
                _ => {}
            }
            result
        },
        Err(e) => panic!(e),
    }
}

