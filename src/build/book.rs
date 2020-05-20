use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;

use ignore::{WalkBuilder};
use mdbook::MDBook;
use log::{info,error,debug,warn};

use crate::{fs,Options,matcher,TEMPLATE};

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
            book.push("book.toml");
            if book.exists() {
                return true
            }
        }
        false
    }

    pub fn add<P: AsRef<Path>>(&mut self, p: P) {
        let b = p.as_ref();
        debug!("ignore book file {}", b.display());
        self.books.push(b.to_path_buf().to_owned());
    }

    fn copy_book(&self, source_dir: &Path, build_dir: PathBuf) {

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
                        let copied = fs::copy(file, output);
                        match copied {
                            Err(e) => {
                                error!("{}", e);
                                std::process::exit(1);
                            },
                            _ => {}
                        }
                    }
                }, Err(e) => {
                    error!("{}", e);
                    std::process::exit(1);
                },
            }
            }
    }

    pub fn build<P: AsRef<Path>>(&self, p: P) {
        let dir = p.as_ref();
        info!("book {}", dir.display());

        let result = MDBook::load(dir);
        match result {
            Ok(mut md) => {
                //println!("{:?}", md.config);

                let theme_dir = matcher::get_theme_dir(&self.options.source, TEMPLATE);
                if theme_dir.exists() {
                    if let Some(s) = theme_dir.to_str() {
                        let theme = s.to_string();

                        if let Err(e) = md.config.set("output.html.theme", theme) {
                            warn!("cannot set book theme {}", e);
                        }
                    } 
                }

                //let theme = md.config.get("output.html.theme").unwrap();
                //debug!("theme {}", theme);

                let built = md.build();
                match built {
                    Ok(_) => {
                        // TODO: copy dir/BOOK -> target output directory
                        let bd = md.config.build.build_dir;
                        let mut src = dir.to_path_buf();
                        src.push(bd);
                        self.copy_book(dir, src);
                    },
                    Err(e) => {
                        error!("{}", e);
                        std::process::exit(1);
                    },
                }
            },
            Err(e) => {
                error!("{}", e);
                std::process::exit(1);
            },
        }
    }

}
