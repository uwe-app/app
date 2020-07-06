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

static INDEX_HTML: &str = "index.html";

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
        } 
    }
}

// Attempt to get an absolute path for a page 
// relative to the source. The resulting href  
// can be passed to the link helper to get a 
// relative path.
pub fn absolute<F: AsRef<Path>>(file: F, config: &Config, options: LinkOptions) -> Result<String> {
    let build = config.build.as_ref().unwrap();
    let src = &build.source;
    let page = file.as_ref();
    if !page.starts_with(src) {
        return Err(
            Error::PageOutsideSource(page.to_path_buf(), src.to_path_buf()));
    }

    let mut rel = page.strip_prefix(src)?.to_path_buf();

    let rewrite_index = build.rewrite_index.is_some() && build.rewrite_index.unwrap();
    if options.rewrite && rewrite_index {
        rel.set_extension("");
        if options.include_index {
            rel.push(INDEX_HTML.to_string());
        }
    }

    if options.transpose {
        if let Some(ref extensions) = config.extension {
            if let Some(ext) = rel.extension() {
                let ext = ext.to_string_lossy().into_owned();
                if let Some(ref map_ext) = extensions.map.get(&ext) {
                    rel.set_extension(map_ext);
                }
            }
        }
    }

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use config::Config;
    use crate::*;

    #[test]
    fn outside_source() -> Result<()> {
        let mut conf: Config = Default::default();
        let source = PathBuf::from("site");
        conf.build.as_mut().unwrap().source = source.clone();
        let page = PathBuf::from("post/article.md");
        let result = absolute(&page, &conf, Default::default());
        assert_eq!(Some(Error::PageOutsideSource(page, source)), result.err());
        Ok(())
    }

    #[test]
    fn absolute_page_extension_rewrite() -> Result<()> {
        let mut conf: Config = Default::default();
        conf.build.as_mut().unwrap().source = PathBuf::from("site");
        let page = PathBuf::from("site/post/article.md");
        let result = absolute(&page, &conf, Default::default())?;
        assert_eq!("/post/article.html", result);
        Ok(())
    }

    #[test]
    fn absolute_page() -> Result<()> {
        let mut conf: Config = Default::default();
        conf.build.as_mut().unwrap().source = PathBuf::from("site");
        let page = PathBuf::from("site/post/article.html");
        let result = absolute(&page, &conf, Default::default())?;
        assert_eq!("/post/article.html", result);
        Ok(())
    }

    #[test]
    fn absolute_rewrite() -> Result<()> {
        let mut conf: Config = Default::default();
        let mut build = conf.build.as_mut().unwrap();
        build.source = PathBuf::from("site");
        build.rewrite_index = Some(true);

        let page = PathBuf::from("site/post/article.html");
        let result = absolute(&page, &conf, Default::default())?;
        assert_eq!("/post/article/", result);
        Ok(())
    }
}
