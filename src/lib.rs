use std::io;
use std::path::Path;
use std::path::PathBuf;

use regex::Regex;
use walkdir::{WalkDir,DirEntry};
use minify::html::minify;
use mdbook::MDBook;
use log::{info,error,debug,warn};

pub mod fs;
pub mod matcher;
pub mod parser;
pub mod template;

use matcher::{FileType,FileMatcher};
use parser::Parser;

pub struct Options {
    pub source: PathBuf,
    pub target: PathBuf,
    pub follow_links: bool,
    pub exclude: Option<Vec<Regex>>,
    pub layout: String,
    pub template: String,
    pub theme: String,
    pub clean: bool,
    pub minify: bool,
}

pub fn build(options: Options) {
    let matcher = FileMatcher::new(&options);
    let finder = Finder::new(&matcher, &options);
    finder.run();
}

fn process_file(
    parser: &mut Parser,
    matcher: &FileMatcher,
    options: &Options,
    file: PathBuf,
    file_type: FileType) -> io::Result<()> {

    let dest = matcher.destination(&file, &file_type, options.clean);

    match file_type {
        FileType::Unknown => {
            return fs::copy(file, dest)
        },
        FileType::Html => {
            info!("html {} -> {}", file.display(), dest.display());
            let result = parser.parse_html(file);
            match result {
                Ok(s) => {
                    if options.minify {
                        return fs::write_string(dest, minify(&s))
                    } else {
                        return fs::write_string(dest, s)
                    }
                },
                Err(e) => return Err(e)
            }
        },
        FileType::Markdown => {
            info!("mark {} -> {}", file.display(), dest.display());
            let result = parser.parse_markdown(file);
            match result {
                Ok(s) => {
                    if options.minify {
                        return fs::write_string(dest, minify(&s))
                    } else {
                        return fs::write_string(dest, s)
                    }
                },
                Err(e) => return Err(e)
            }
        },
        FileType::Ignored | FileType::Private | FileType::Template => {
            // Ignore templates here as they are located and 
            // used during the parsing and rendering process
            debug!("noop {}", file.display());
        },
    }

    Ok(())
}

pub struct Finder<'a> {
    matcher: &'a FileMatcher<'a>,
    options: &'a Options,
}

impl<'a> Finder<'a> {

    pub fn new(matcher: &'a FileMatcher, options: &'a Options) -> Self {
        Finder{matcher, options} 
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

    fn book(&self, dir: &Path) {
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

    fn handle(&self, entry: &DirEntry) -> bool {
        let path = entry.path().clone();
        if path.is_dir() {
            let buf = &path.to_path_buf();
            // Can prevent recursing if a directory pattern matches
            if self.matcher.is_excluded(buf) {
                return false 
            }

            if self.matcher.is_theme(&self.options.source, buf) {
                return false
            }
            let mut book = buf.clone();
            book.push("book.toml");
            if book.exists() {
                self.book(book.parent().unwrap());
                return false
            }
        }
        true
    }

    // Find files in an input directory to process and invoke the callback 
    // for each matched file.
    fn walk<T>(&self, mut callback: T) where T: FnMut(PathBuf, FileType) {
        let walker = WalkDir::new(self.options.source.clone())
            .follow_links(self.options.follow_links)
            .into_iter();

        let iter = walker.filter_entry(|e| self.handle(e));
        for entry in iter {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                let file = entry.path().to_path_buf();
                let file_type = self.matcher.get_type(&file);
                callback(file, file_type)
            }
        }
    }

    // Find files and process each entry.
    pub fn run(&self) {
        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let mut parser = Parser::new(self.options);

        let mut templates = self.options.source.clone();
        templates.push(&self.options.template);
        if let Err(e) = parser.register_templates_directory(".hbs", templates.as_path()) {
            error!("{}", e);
            std::process::exit(1);
        }

        self.walk(|file, file_type| {
            let result = process_file(&mut parser, &self.matcher, &self.options, file, file_type);
            match result {
                Err(e) => {
                    error!("{}", e);
                    std::process::exit(1);
                },
                _ => {},
            }
        });
    }
}
