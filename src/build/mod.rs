use std::io;
use std::path::PathBuf;

use minify::html::minify;
use log::{info,error,debug};
use ignore::WalkBuilder;

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

    // Find files and process each entry.
    pub fn run(&mut self) {
        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let mut parser = Parser::new(self.options);

        let mut templates = self.options.source.clone();
        templates.push(&self.options.template);
        if let Err(e) = parser.register_templates_directory(".hbs", templates.as_path()) {
            error!("{}", e);
            std::process::exit(1);
        }

        for result in WalkBuilder::new(&self.options.source)
            .follow_links(self.options.follow_links)
            .hidden(false)
            .build() {

            match result {
                Ok(entry) => {
                    let path = entry.path();

                    if self.book.contains_file(&path) {
                        continue;
                    }

                    //println!("ENTRY");
                    if path.is_dir() && self.book.is_book_dir(&path) {
                        // Add the book so we can skip processing of descendants
                        self.book.add(&path);

                        // Build the book
                        self.book.build(&path);
                    } else if path.is_file() {
                        //println!("{:?}", entry);

                        let file = entry.path().to_path_buf();
                        let file_type = self.matcher.get_type(&file);

                        let result = process_file(&mut parser, &self.matcher, &self.options, file, file_type);
                        match result {
                            Err(e) => {
                                error!("{}", e);
                                std::process::exit(1);
                            },
                            _ => {},
                        }
                    }
                },
                Err(e) => {
                    error!("{}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
