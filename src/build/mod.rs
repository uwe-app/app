use std::io;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use walkdir::{WalkDir,DirEntry};
use minify::html::minify;
use log::{info,error,debug};
use gitignore::File;

mod book;

use super::fs;
use super::Options;
use super::matcher::{FileType,FileMatcher};
use super::parser::Parser;
use book::BookBuilder;

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

pub struct Builder<'a> {
    matcher: &'a FileMatcher<'a>,
    options: &'a Options,
    book: BookBuilder<'a>,
}

impl<'a> Builder<'a> {

    pub fn new(matcher: &'a FileMatcher, options: &'a Options) -> Self {
        let book = BookBuilder::new(matcher, options);
        Builder{matcher, options, book} 
    }


    fn is_ignored(&self) -> bool {
    
        //if let Some(ignores_file) = self.get_ignores_file_parent(&file) {
            //if ignores_file.exists() {
                //let ignores = File::new(&ignores_file).unwrap();
                //if ignores.is_excluded(file) {
                    //println!("EXCLUDED THE FILE {}", file.display()); 
                //}

                ////if self.ignores.contains_key(&ignores_file) {
                    ////println!("USE IGNORES FILE");
                ////} else {
                    ////println!("CREATE IGNORES FILE");

                    ////let tmp = &ignores_file.as_path();
                    ////let file = File::new(&tmp).unwrap();
                    //////self.ignores.insert(ignores_file, file);
                ////}

            //}
        //}

        false
    }

    //pub fn get_ignores_file_parent<P: AsRef<Path>>(&self, f: P) -> Option<PathBuf> {
        //if let Some(parent) = f.as_ref().parent() {
            //let mut ignores = parent.to_path_buf();
            //ignores.push(".gitignore");
            //return Some(ignores)
        //}
        //None
    //}

    fn handle(&self, entry: &DirEntry, ignores: &'a mut HashMap<PathBuf, File>) -> bool {
        let path = entry.path();
        if path.is_dir() {

            let mut ignores_file = entry.path().to_path_buf();
            ignores_file.push(".gitignore");

            if ignores_file.exists() {
                let tmp = &ignores_file.clone();
                let gf = File::new(tmp.as_path());
                if let Ok(file) = gf {
                    //ignores.insert(tmp.to_owned(), file);
                }

            }

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
                self.book.build(book.parent().unwrap());
                return false
            }
        }
        true
    }

    // Find files in an input directory to process and invoke the callback 
    // for each matched file.
    fn walk<T>(&self, ignores: &'a mut HashMap<PathBuf, File>, mut callback: T) where T: FnMut(PathBuf, FileType) {
        let walker = WalkDir::new(self.options.source.clone())
            .follow_links(self.options.follow_links)
            .into_iter();

        let iter = walker.filter_entry(|e| self.handle(e, ignores));
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

        // Store ignore files found in directories
        let mut ignores: HashMap<PathBuf, File> = HashMap::new();

        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let mut parser = Parser::new(self.options);

        let mut templates = self.options.source.clone();
        templates.push(&self.options.template);
        if let Err(e) = parser.register_templates_directory(".hbs", templates.as_path()) {
            error!("{}", e);
            std::process::exit(1);
        }

        self.walk(&mut ignores, |file, file_type| {
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
