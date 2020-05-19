use std::path::Path;
use std::path::PathBuf;

use walkdir::{WalkDir};
use mdbook::MDBook;
use log::{info,error,debug,warn};

use crate::fs;
use crate::Options;
use crate::matcher::FileMatcher;

pub struct BookBuilder<'a> {
    matcher: &'a FileMatcher<'a>,
    options: &'a Options,
}

impl<'a> BookBuilder<'a> {

    pub fn new(matcher: &'a FileMatcher, options: &'a Options) -> Self {
        BookBuilder{matcher, options} 
    }

    fn copy_book(&self, source_dir: &Path, build_dir: PathBuf) {

        // Jump some hoops to bypass the book build_dir
        let relative = source_dir.strip_prefix(&self.options.source).unwrap();
        let mut base = self.options.target.clone();
        base.push(relative);

        let walker = WalkDir::new(&build_dir)
            .follow_links(self.options.follow_links);
        for entry in walker {
            let entry = entry.unwrap();

            if self.matcher.is_excluded(&entry.path()) {
                debug!("noop {}", entry.path().display());
            } else {
                if entry.file_type().is_file() {
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
            }

        }
    }

    pub fn build(&self, dir: &Path) {
        info!("book {}", dir.display());

        let result = MDBook::load(dir);
        match result {
            Ok(mut md) => {
                //println!("{:?}", md.config);

                let mut theme = self.options.theme.clone();

                if theme.is_empty() {
                    let theme_dir = self.matcher.get_theme_dir(&self.options.source);
                    if theme_dir.exists() {
                        if let Some(s) = theme_dir.to_str() {
                            theme = s.to_string();
                        } 
                    }
                }

                if let Err(e) = md.config.set("output.html.theme", theme) {
                    warn!("cannot set book theme {}", e);
                }

                let theme = md.config.get("output.html.theme").unwrap();

                debug!("theme {}", theme);

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
