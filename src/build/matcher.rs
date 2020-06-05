use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug)]
pub enum FileType {
    Markdown,
    Template,
    Private,
    Unknown,
}

use crate::{
    Error,
    HTML,
    INDEX_HTML,
    INDEX_STEM,
    LAYOUT_HBS,
    DATA_TOML,
    MD,
    PARSE_EXTENSIONS,
    TEMPLATE,
    THEME,
};

use super::generator;
use crate::config::ExtensionsConfig;
use crate::utils;

fn resolve_dir_index<P: AsRef<Path>>(file: P) -> Option<PathBuf> {
    let mut buf = file.as_ref().to_path_buf();
    buf.push(INDEX_STEM);
    for ext in PARSE_EXTENSIONS.iter() {
        buf.set_extension(ext);
        if buf.exists() {
            return Some(buf)
        }
    }
    None
}

pub fn resolve_parent_index<P: AsRef<Path>>(file: P) -> Option<PathBuf> {
    if let Some(parent) = file.as_ref().parent() {

        // Not an index file so a single level is sufficient
        if !is_index(&file) {
            return resolve_dir_index(&parent);
        // Otherwise go back down one more level
        } else {
            if let Some(parent) = parent.parent() {
                return resolve_dir_index(&parent);
            }
        }

    }
    None
}

// Try to find a generator file for the given URL
pub fn lookup_generator(href: &str, clean_url: bool) -> Option<PathBuf> {
    let mut url = href.to_string().clone();
    url = utils::url::trim_slash(&url).to_owned();

    // Try to match against generated output files.
    //
    // For these cases there are no source files on disc with 
    // a direct mapping to output files as they are generated
    // and this code can be called (via the `link` helper) before
    // output has been generated so we cannot compare to output
    // destination files.
    let mapping = generator::GENERATOR_MAPPING.lock().unwrap();
    for (_, map) in mapping.iter() {
        let dest = Path::new(&map.destination);


        // Now try to match on generated document id
        for id in &map.ids {
            let mut page = dest.to_path_buf();
            if clean_url {
                page.push(id);
                let mut target = utils::url::to_url_lossy(&page);
                if target == url {
                    return Some(page)
                }

                page.push(INDEX_HTML);

                target = utils::url::to_url_lossy(&page);
                if target == url {
                    return Some(page)
                }
            } else {
                page.push(id);
                page.set_extension(HTML);
                let target = utils::url::to_url_lossy(&page);
                if target == url {
                    return Some(page)
                }
            }
        }
    }
    None
}

// Try to find a source file for the given URL
pub fn lookup<P: AsRef<Path>>(base: P, href: &str, clean_url: bool) -> Option<PathBuf> {

    let mut url = href.to_string().clone();
    url = utils::url::trim_slash(&url).to_owned();

    let is_dir = utils::url::is_dir(&url);

    let mut buf = base.as_ref().to_path_buf();
    buf.push(&utils::url::to_path_separator(&url));

    // Check if the file exists directly
    if buf.exists() {
        return Some(buf)
    }

    // Check index pages
    if is_dir {
        let mut idx = base.as_ref().to_path_buf();
        idx.push(&utils::url::to_path_separator(&url));
        idx.push(INDEX_STEM);
        for ext in PARSE_EXTENSIONS.iter() {
            idx.set_extension(ext);
            if idx.exists() {
                return Some(buf)
            }
        }
    }

    // Check for lower-level files that could map 
    // to index pages
    if clean_url && is_dir {
        for ext in PARSE_EXTENSIONS.iter() {
            buf.set_extension(ext);
            if buf.exists() {
                return Some(buf)
            }
        }
    } 

    None
}

pub fn source_exists<P: AsRef<Path>>(base: P, href: &str, clean_url: bool) -> bool {
    lookup(&base, href, clean_url).is_some() || lookup_generator(href, clean_url).is_some()
}

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

// FIXME: test on extensions
pub fn collides<P: AsRef<Path>>(file: P, file_type: &FileType) -> (bool, PathBuf) {
    let mut other = file.as_ref().to_path_buf(); 
    match file_type {
        FileType::Markdown => {
            other.set_extension(HTML);
            return (other.exists(), other)
        },
        FileType::Template => {
            other.set_extension(MD);
            return (other.exists(), other)
        }
        _ => return (false, Path::new("").to_path_buf())
    }
}

fn get_type_extension<P: AsRef<Path>>(p: P, extensions: &ExtensionsConfig) -> FileType {
    let file = p.as_ref();
    if let Some(ext) = file.extension() {
        let ext = ext.to_string_lossy().into_owned();
        if extensions.parse.contains(&ext) {
            if extensions.markdown.contains(&ext) {
                return FileType::Markdown
            } else {
                return FileType::Template
            }
        }
    }
    FileType::Unknown
}

pub fn get_type<P: AsRef<Path>>(p: P, extensions: &ExtensionsConfig) -> FileType {
    let file = p.as_ref();
    match file.file_name() {
        Some(nm) => {
            if let Some(nm) = nm.to_str() {
                if nm == LAYOUT_HBS || nm == DATA_TOML {
                    return FileType::Private
                } else {
                    return get_type_extension(p, extensions)
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

// Build the direct destination file path.
pub fn direct_destination<P: AsRef<Path>>(
    source: P,
    target: P,
    file: P,
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
            let result = target.as_ref().clone().join(relative);
            return Ok(result)
        }
        Err(e) => return Err(Error::from(e))
    }
}

// Build the destination file path and update the file extension.
pub fn destination<P: AsRef<Path>>(
    source: P,
    target: P,
    file: P,
    file_type: &FileType,
    extensions: &ExtensionsConfig,
    clean_urls: bool,
) -> Result<PathBuf, Error> {
    let pth = file.as_ref().to_path_buf().clone();
    let result = direct_destination(source, target, file);
    match result {
        Ok(mut result) => {
            match file_type {
                FileType::Markdown | FileType::Template => {

                    if let Some(ext) = pth.extension() {
                        let ext = ext.to_string_lossy().into_owned();
                        for (k, v) in &extensions.map {
                            if ext == *k {
                                result.set_extension(v);
                                break;
                            }
                        }
                    }

                    if clean_urls {
                        if let Some(res) = clean(pth.as_path(), result.as_path()) {
                            result = res;
                        }
                    }

                }
                _ => {}
            }
            return Ok(result)
        },
        Err(e) => return Err(e)
    }

}
