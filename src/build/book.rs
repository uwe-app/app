use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use log::{debug, info, warn};
use mdbook::MDBook;

use crate::build::loader;
use crate::build::matcher;

use crate::{
    utils,
    Error,
    ROOT_TABLE_KEY,
    DRAFT_KEY,
    BOOK_THEME_KEY,
    BOOK_TOML
};

use super::context::Context;

pub struct BookBuilder<'a> {
    books: Vec<PathBuf>,
    context: &'a Context,
}

impl<'a> BookBuilder<'a> {
    pub fn new(context: &'a Context) -> Self {
        let books: Vec<PathBuf> = Vec::new();
        BookBuilder { books, context }
    }

    pub fn contains_file<P: AsRef<Path>>(&self, p: P) -> bool {
        let f = p.as_ref();
        for b in self.books.iter() {
            if f.starts_with(b.as_path()) {
                debug!("ignore book file {}", f.display());
                return true;
            }
        }
        false
    }

    pub fn get_book_config<P: AsRef<Path>>(&self, p: P) -> PathBuf {
        let mut book = p.as_ref().to_path_buf();
        book.push(BOOK_TOML);
        book
    }

    pub fn is_book_dir<P: AsRef<Path>>(&self, p: P) -> bool {
        let book = self.get_book_config(p);
        if book.exists() {
            return true;
        }
        false
    }

    pub fn add<P: AsRef<Path>>(&mut self, p: P) {
        let b = p.as_ref();
        self.books.push(b.to_path_buf().to_owned());
    }

    fn copy_book(&self, source_dir: &Path, build_dir: PathBuf) -> Result<(), Error> {
        // Jump some hoops to bypass the book build_dir
        let relative = source_dir.strip_prefix(&self.context.options.source).unwrap();
        let mut base = self.context.options.target.clone();
        base.push(relative);

        for result in WalkBuilder::new(&build_dir)
            .follow_links(self.context.config.build.follow_links)
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
                        if let Err(e) = utils::copy(file, output) {
                            return Err(Error::IoError(e));
                        }
                    }
                }
                Err(e) => return Err(Error::IgnoreError(e)),
            }
        }

        Ok(())
    }

    pub fn build<P: AsRef<Path>>(&self, p: P) -> Result<(), Error> {
        let dir = p.as_ref();

        let mut is_draft = false;
        if self.context.options.release {
            let conf_result = loader::load_toml_to_json(self.get_book_config(&dir));
            match conf_result {
                Ok(map) => {
                    if let Some(site) = map.get(ROOT_TABLE_KEY) {
                        if let Some(draft) = site.get(DRAFT_KEY) {
                            if let Some(val) = draft.as_bool() {
                                is_draft = val;
                            }
                        }

                    }
                },
                Err(e) => return Err(e)
            }
        }

        if is_draft {
            return Ok(())
        }

        info!("book {}", dir.display());

        let result = MDBook::load(dir);
        match result {
            Ok(mut md) => {
                //println!("{:?}", md.config);

                let theme_dir = matcher::get_theme_dir(&self.context.options.source);
                if theme_dir.exists() {
                    if let Some(s) = theme_dir.to_str() {
                        if let Err(e) = md.config.set(BOOK_THEME_KEY, s) {
                            warn!("cannot set book theme {}", e);
                        }
                    }
                }

                let built = md.build();
                match built {
                    Ok(_) => {
                        let bd = md.config.build.build_dir;
                        let mut src = dir.to_path_buf();
                        src.push(bd);
                        return self.copy_book(dir, src);
                    }
                    Err(e) => return Err(Error::BookError(e)),
                }
            }
            Err(e) => return Err(Error::BookError(e)),
        }
    }
}
