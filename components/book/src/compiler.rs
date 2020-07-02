use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use log::{info, warn};
use mdbook::MDBook;

use ignore::WalkBuilder;

use config::Config;
use utils;

use super::Error;

static BOOK_TOML: &str = "book.toml";
static BOOK_THEME_KEY: &str = "output.html.theme";

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

    fn copy_book(&self, config: &Config, source_dir: &Path, build_dir: PathBuf) -> Result<(), Error> {
        // Jump some hoops to bypass the book build_dir
        let relative = source_dir.strip_prefix(&self.source)?;
        let mut base = self.target.clone();
        base.push(relative);

        let build = config.build.as_ref().unwrap();
        let follow_links = build.follow_links.is_some() && build.follow_links.unwrap();

        for result in WalkBuilder::new(&build_dir)
            .follow_links(follow_links)
            .build()
        {
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

    pub fn locate<P: AsRef<Path>>(&self, config: &Config, p: P) -> Option<MDBook> {
        let base = self.source.clone();
        let pth = p.as_ref().to_path_buf();
        if let Ok(md) = self.load(config, &base, &pth, None) {
            return Some(md)
        }
        None
    }

    pub fn load<P: AsRef<Path>>(
        &self,
        config: &Config,
        base:P,
        p: P,
        livereload: Option<String>) -> Result<MDBook, Error> {

        let dir = p.as_ref().to_path_buf();
        info!("load {}", dir.display());

        let result = MDBook::load(dir);
        match result {
            Ok(mut md) => {
                let theme = config.get_book_theme_path(base.as_ref());
                if let Some(theme_dir) = theme {
                    if theme_dir.exists() && theme_dir.is_dir() {
                        if let Some(s) = theme_dir.to_str() {
                            md.config.set(BOOK_THEME_KEY, s)?;
                        }
                    } else {
                        warn!("Missing book theme directory '{}'", theme_dir.display());
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
        &self, config: &Config, md: MDBook, rel: P, p: P) -> Result<(), Error> {
        let dir = p.as_ref();
        if let Some(ref book) = config.book {
            if let Some(cfg) = book.find(rel.as_ref()) {
                info!("build {}", dir.display());

                let draft = cfg.draft.is_some() && cfg.draft.unwrap();
                if draft && self.release {
                    return Ok(())
                }

                let built = md.build();
                match built {
                    Ok(_) => {
                        let bd = &md.config.build.build_dir;
                        let mut src = dir.to_path_buf();
                        src.push(bd);
                        return self.copy_book(config, dir, src)
                    }
                    Err(e) => return Err(Error::from(e)),
                }
            } else {
                return Err(
                    Error::new(
                        format!("No book found for {}", dir.display())))
            }
        }
        Ok(())
    }

    pub fn build<P: AsRef<Path>>(
        &mut self,
        config: &Config,
        base: P,
        p: P,
        livereload: Option<String>) -> Result<(), Error> {

        let pth = p.as_ref().to_path_buf().clone();
        let rel = pth.strip_prefix(base.as_ref())?;
        // NOTE: mdbook requires a reload before a build
        let book = self.load(config, base, p, livereload)?;
        self.compile(config, book, rel, &pth)
    }

    pub fn all<P: AsRef<Path>>(
        &mut self,
        config: &Config,
        base: P,
        livereload: Option<String>) -> Result<(), Error> {

        if let Some(ref book) = config.book {
            let paths = book.get_paths(base.as_ref());
            for p in paths {
                self.build(config, base.as_ref(), &p, livereload.clone())?;
            }
        }
        Ok(())
    }
}
