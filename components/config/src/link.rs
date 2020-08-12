use std::path::{Path, PathBuf};

use super::{Error, Result, RuntimeOptions};
use super::file::FileInfo;

use super::config::INDEX_STEM;

pub struct LinkOptions {
    // Convert paths to forward slashes
    pub slashes: bool,
    // Use a leading slash
    pub leading: bool,
    // Use a trailing slash
    pub trailing: bool,
    // Transpose the file extension
    pub transpose: bool,
    // Rewrite to index links when rewrite_index
    pub rewrite: bool,
    // Include index.html when rewrite is active
    pub include_index: bool,
    // Strip this prefix
    pub strip: Option<PathBuf>,
}

impl Default for LinkOptions {
    fn default() -> Self {
        Self {
            slashes: true,
            leading: true,
            trailing: true,
            transpose: true,
            rewrite: true,
            include_index: false,
            strip: None,
        }
    }
}

fn is_home_index<P: AsRef<Path>>(p: P) -> bool {
    let rel = p.as_ref();
    if rel.components().count() == 1 {
        if let Some(stem) = rel.file_stem() {
            if stem == INDEX_STEM {
                return true;
            }
        }
    }
    false
}

fn to_href<R: AsRef<Path>>(rel: R, options: LinkOptions) -> Result<String> {
    let rel = rel.as_ref();

    let mut href = if options.leading {
        "/".to_string()
    } else {
        "".to_string()
    };
    let value = if options.slashes {
        utils::url::to_href_separator(&rel)
    } else {
        rel.to_string_lossy().into_owned()
    };

    href.push_str(&value);

    if options.trailing && rel.extension().is_none() {
        href.push('/');
    }
    Ok(href)
}

// Attempt to get an absolute URL path
// for an asset relative to a source.
/*
pub fn asset<F: AsRef<Path>, S: AsRef<Path>>(file: F, source: S, options: LinkOptions) -> Result<String> {
    let file = file.as_ref();
    let source = source.as_ref();
    if !file.starts_with(source) {
        return Err(
            Error::PageOutsideSource(
                file.to_path_buf(), source.to_path_buf()));
    }
    to_href(file.strip_prefix(source)?, options)
}
*/

pub fn relative<P: AsRef<Path>, B: AsRef<Path>>(
    href: &str,
    path: P,
    base: B,
    opts: &RuntimeOptions) -> Result<String> {

    let rel = path.as_ref().strip_prefix(base.as_ref())?;

    let types = opts.settings.types.as_ref().unwrap();
    let include_index = opts.settings.should_include_index();
    let rewrite_index = opts.settings.should_rewrite_index();

    let up = "../";
    let mut value: String = "".to_string();
    if let Some(p) = rel.parent() {
        if rewrite_index && FileInfo::is_clean(path.as_ref(), types) {
            value.push_str(up);
        }
        for _ in p.components() {
            value.push_str(up);
        }
    }

    value.push_str(&href);

    if include_index && (value.ends_with("/") || value == "") {
        value.push_str(super::INDEX_HTML);
    }

    if !rewrite_index && value == "" {
        value = up.to_string();
    }
    Ok(value)
}

// Attempt to get an absolute URL path for a page
// relative to the source. The resulting href
// can be passed to the link helper to get a
// relative path.
pub fn absolute<F: AsRef<Path>>(
    file: F,
    opts: &RuntimeOptions,
    options: LinkOptions,
) -> Result<String> {
    let src = if let Some(ref source) = options.strip {
        source
    } else {
        &opts.source
    };

    let page = file.as_ref();
    if !page.starts_with(src) {
        return Err(Error::PageOutsideSource(
            page.to_path_buf(),
            src.to_path_buf(),
        ));
    }

    let mut rel = page.strip_prefix(src)?.to_path_buf();

    if is_home_index(&rel) {
        return Ok("/".to_string());
    }

    let rewrite_index = opts.settings.should_rewrite_index();
    if options.rewrite && rewrite_index {
        rel.set_extension("");
        if let Some(stem) = rel.file_stem() {
            if options.include_index {
                if stem == INDEX_STEM {
                    rel.set_extension(crate::HTML);
                } else {
                    rel.push(crate::INDEX_HTML);
                }
            } else {
                if stem == INDEX_STEM {
                    if let Some(parent) = rel.parent() {
                        rel = parent.to_path_buf();
                    }
                }
            }
        }
    }

    if options.transpose {
        if let Some(ref types) = opts.settings.types {
            if let Some(ext) = rel.extension() {
                let ext = ext.to_string_lossy().into_owned();
                if let Some(ref map_ext) = types.map().get(&ext) {
                    rel.set_extension(map_ext);
                }
            }
        }
    }

    to_href(rel, options)
}

#[cfg(test)]
mod tests {
    use crate::link::*;
    use crate::{Config, RuntimeOptions};
    use std::path::PathBuf;

    #[test]
    fn outside_source() -> Result<()> {
        let mut opts: RuntimeOptions = Default::default();
        let source = PathBuf::from("site");
        opts.source = source.clone();
        let page = PathBuf::from("post/article.md");
        let result = absolute(&page, &opts, Default::default());
        // TODO: restore this - requires PartialEq on Error
        //assert_eq!(Some(Error::PageOutsideSource(page, source)), result.err());
        Ok(())
    }

    #[test]
    fn absolute_page_extension_rewrite() -> Result<()> {
        let mut opts: RuntimeOptions = Default::default();
        opts.source = PathBuf::from("site");
        let page = PathBuf::from("site/post/article.md");
        let result = absolute(&page, &opts, Default::default())?;
        assert_eq!("/post/article.html", result);
        Ok(())
    }

    #[test]
    fn absolute_page() -> Result<()> {
        let mut opts: RuntimeOptions = Default::default();
        opts.source = PathBuf::from("site");
        let page = PathBuf::from("site/post/article.html");
        let result = absolute(&page, &opts, Default::default())?;
        assert_eq!("/post/article.html", result);
        Ok(())
    }

    #[test]
    fn absolute_rewrite() -> Result<()> {
        let mut opts: RuntimeOptions = Default::default();
        opts.source = PathBuf::from("site");
        opts.settings.rewrite_index = Some(true);

        let page = PathBuf::from("site/post/article.html");
        let result = absolute(&page, &opts, Default::default())?;
        assert_eq!("/post/article/", result);
        Ok(())
    }
}
