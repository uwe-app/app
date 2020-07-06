use std::path::Path;
use std::path::PathBuf;

use thiserror::Error;

use config::Config;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("Page {0} is outside the source directory {1}")]
    PageOutsideSource(PathBuf, PathBuf),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),
}

type Result<T> = std::result::Result<T, Error>;

// Attempt to get an absolute path for a page 
// relative to the source. The resulting href  
// can be passed to the link helper to get a 
// relative path.
pub fn absolute<P: AsRef<Path>>(source: P, file: P, config: &Config) -> Result<String> {
    let src = source.as_ref();
    let page = file.as_ref();
    if !page.starts_with(src) {
        return Err(
            Error::PageOutsideSource(page.to_path_buf(), src.to_path_buf()));
    }

    let mut rel = page.strip_prefix(src)?.to_path_buf();
    if let Some(ref extensions) = config.extension {
        if let Some(ext) = rel.extension() {
            let ext = ext.to_string_lossy().into_owned();
            if let Some(ref map_ext) = extensions.map.get(&ext) {
                rel.set_extension(map_ext);
            }
        }
    }

    let mut href = "/".to_string();
    href.push_str(&utils::url::to_href_separator(rel));
    Ok(href)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use config::Config;
    use crate::*;

    #[test]
    fn outside_source() -> Result<()> {
        let conf: Config = Default::default();
        let source = PathBuf::from("site");
        let page = PathBuf::from("post/article.md");
        let result = absolute(&source, &page, &conf);
        assert_eq!(Some(Error::PageOutsideSource(page, source)), result.err());
        Ok(())
    }

    #[test]
    fn absolute_page_extension_rewrite() -> Result<()> {
        let conf: Config = Default::default();
        let source = PathBuf::from("site");
        let page = PathBuf::from("site/post/article.md");
        let result = absolute(&source, &page, &conf)?;
        assert_eq!("/post/article.html", result);
        Ok(())
    }

    #[test]
    fn absolute_page() -> Result<()> {
        let conf: Config = Default::default();
        let source = PathBuf::from("site");
        let page = PathBuf::from("site/post/article.html");
        let result = absolute(&source, &page, &conf)?;
        assert_eq!("/post/article.html", result);
        Ok(())
    }
}
