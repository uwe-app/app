use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use log::{info, warn};
use mdbook::MDBook;

use ignore::WalkBuilder;

use config::Config;
use utils;

use super::{Error, Result};

static BOOK_TOML: &str = "book.toml";
static BOOK_THEME_KEY: &str = "output.html.theme";

#[derive(Debug)]
pub struct BookCompiler {
    source: PathBuf,
    target: PathBuf,
    release: bool,
}

impl BookCompiler {
    pub fn new(source: PathBuf, target: PathBuf, release: bool) -> Self {
        BookCompiler {
            source,
            target,
            release,
        }
    }

    pub fn get_book_config<P: AsRef<Path>>(&self, p: P) -> PathBuf {
        let mut book = p.as_ref().to_path_buf();
        book.push(BOOK_TOML);
        book
    }

    fn copy_book(&self, source_dir: &Path, build_dir: PathBuf) -> Result<()> {
        // Jump some hoops to bypass the book build_dir
        let relative = source_dir.strip_prefix(&self.source)?;
        let mut base = self.target.clone();
        base.push(relative);

        for result in WalkBuilder::new(&build_dir).follow_links(true).build() {
            match result {
                Ok(entry) => {
                    if entry.path().is_file() {
                        let file = entry.path().to_path_buf();
                        // Get a relative file and append it to the correct output base directory
                        let dest = file.strip_prefix(&build_dir).unwrap();
                        let mut output = base.clone();
                        output.push(dest);

                        // TODO: minify files with HTML file extension

                        // Copy the file content
                        if let Err(e) = utils::fs::copy(file, output) {
                            return Err(Error::from(e));
                        }
                    }
                }
                Err(e) => return Err(Error::from(e)),
            }
        }

        Ok(())
    }

    pub fn locate<P: AsRef<Path>>(
        &self,
        config: &Config,
        p: P,
    ) -> Option<MDBook> {
        let pth = p.as_ref().to_path_buf();
        if let Ok(md) = self.load(config, &pth, None) {
            return Some(md);
        }
        None
    }

    pub fn load<P: AsRef<Path>>(
        &self,
        config: &Config,
        p: P,
        livereload: Option<String>,
    ) -> Result<MDBook> {
        let dir = p.as_ref().to_path_buf();

        let result = MDBook::load(dir);
        match result {
            Ok(mut md) => {
                let theme = config.get_book_theme_path(&self.source);
                if let Some(theme_dir) = theme {
                    if theme_dir.exists() && theme_dir.is_dir() {
                        if let Some(s) = theme_dir.to_str() {
                            md.config.set(BOOK_THEME_KEY, s)?;
                        }
                    } else {
                        warn!(
                            "Missing book theme directory '{}'",
                            theme_dir.display()
                        );
                    }
                }

                if let Some(ref livereload_url) = livereload {
                    md.config
                        .set("output.html.livereload-url", livereload_url)?;
                }

                Ok(md)
            }
            Err(e) => return Err(Error::from(e)),
        }
    }

    fn compile<P: AsRef<Path>>(
        &self,
        config: &Config,
        md: MDBook,
        rel: P,
        p: P,
    ) -> Result<()> {
        let dir = p.as_ref();
        if let Some(ref book) = config.book {
            if let Some(cfg) = book.find(&rel.as_ref().to_path_buf()) {
                let draft = cfg.draft.is_some() && cfg.draft.unwrap();
                if draft && self.release {
                    return Ok(());
                }

                let built = md.build();
                match built {
                    Ok(_) => {
                        let bd = &md.config.build.build_dir;
                        let mut src = dir.to_path_buf();
                        src.push(bd);
                        return self.copy_book(dir, src);
                    }
                    Err(e) => return Err(Error::from(e)),
                }
            } else {
                return Err(Error::NoBookFound(dir.to_path_buf()));
            }
        }
        Ok(())
    }

    pub fn build<P: AsRef<Path>>(
        &self,
        config: &Config,
        p: P,
        livereload: Option<String>,
    ) -> Result<()> {
        let pth = p.as_ref().to_path_buf().clone();
        let rel = pth.strip_prefix(&self.source)?;
        // NOTE: mdbook requires a reload before a build
        info!("Build {}", pth.display());
        let book = self.load(config, p, livereload)?;
        self.compile(config, book, rel, &pth)
    }

    pub fn all(
        &self,
        config: &Config,
        livereload: Option<String>,
    ) -> Result<()> {
        if let Some(ref book) = config.book {
            let paths = book.get_paths(&self.source);
            for p in paths {
                self.build(config, &p, livereload.clone())?;
            }
        }
        Ok(())
    }
}
