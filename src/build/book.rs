use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;

use ignore::{WalkBuilder};
use mdbook::MDBook;
use log::{info,debug,warn};

use crate::{
    fs,
    Error,
    Options,
    matcher,
    BOOK_TOML,
    BOOK_THEME_KEY
};

pub struct BookBuilder<'a> {
    books: Vec<PathBuf>,
    options: &'a Options,
}

impl<'a> BookBuilder<'a> {

    pub fn new(options: &'a Options) -> Self {
        let books: Vec<PathBuf> = Vec::new();
        BookBuilder{books, options} 
    }

    pub fn contains_file<P: AsRef<Path>>(&self, p: P) -> bool {
        let f = p.as_ref();
        for b in self.books.iter() {
            if f.starts_with(b.as_path()) {
                debug!("ignore book file {}", f.display());
                return true
            }
        }
        false
    }

    pub fn is_book_dir<P: AsRef<Path>>(&self, p: P) -> bool {
        let e = p.as_ref();
        if e.is_dir() {
            let parent = e.to_path_buf();
            let mut book = parent.clone();
            book.push(BOOK_TOML);
            if book.exists() {
                return true
            }
        }
        false
    }

    pub fn add<P: AsRef<Path>>(&mut self, p: P) {
        let b = p.as_ref();
        self.books.push(b.to_path_buf().to_owned());
    }

    fn copy_book(&self, source_dir: &Path, build_dir: PathBuf) -> Result<(), Error> {

        // Jump some hoops to bypass the book build_dir
        let relative = source_dir.strip_prefix(&self.options.source).unwrap();
        let mut base = self.options.target.clone();
        base.push(relative);

        for result in WalkBuilder::new(&build_dir).follow_links(self.options.follow_links).build() {

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
                        if let Err(e) = fs::copy(file, output) {
                            return Err(Error::IoError(e))
                        }
                    }
                },
                Err(e) => return Err(Error::IgnoreError(e)),
            }
        }

        Ok(())
    }

    pub fn build<P: AsRef<Path>>(&self, p: P) -> Result<(), Error> {
        let dir = p.as_ref();
        info!("book {}", dir.display());

        let result = MDBook::load(dir);
        match result {
            Ok(mut md) => {
                //println!("{:?}", md.config);

                let theme_dir = matcher::get_theme_dir(&self.options.source);
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
                        return self.copy_book(dir, src)
                    },
                    Err(e) => return Err(Error::BookError(e)),
                }
            },
            Err(e) => return Err(Error::BookError(e)),
        }
    }

}
