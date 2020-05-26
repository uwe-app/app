use std::path::PathBuf;

use ignore::WalkBuilder;
use log::{debug, info, error};

pub mod book;
pub mod loader;
pub mod helpers;
pub mod matcher;
pub mod parser;
pub mod template;

use super::{utils, Error, BuildOptions, TEMPLATE, TEMPLATE_EXT, DATA_TOML, LAYOUT_HBS};
use book::BookBuilder;
use matcher::FileType;
use parser::Parser;

#[derive(Debug)]
pub struct Invalidation {
    data: bool,
    layout: bool,
    paths: Vec<PathBuf>,
}

pub struct Builder<'a> {
    options: &'a BuildOptions,
    book: BookBuilder<'a>,
    parser: Parser<'a>,
}

impl<'a> Builder<'a> {
    pub fn new(options: &'a BuildOptions) -> Self {
        let book = BookBuilder::new(options);

        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let parser = Parser::new(options);

        Builder {
            options,
            book,
            parser,
        }
    }

    fn process_file(&mut self, file: PathBuf, file_type: FileType) -> Result<(), Error> {
        let dest = matcher::destination(
            &self.options.source,
            &self.options.target,
            &file,
            &file_type,
            self.options.clean_url,
        )?;

        match file_type {
            FileType::Unknown => return utils::copy(file, dest).map_err(Error::from),
            FileType::Markdown | FileType::Html => {
                let mut data = loader::compute(&file);

                if utils::is_draft(&data, self.options) {
                    return Ok(())
                }

                info!("{} -> {}", file.display(), dest.display());
                let result = self.parser.parse(file, file_type, &mut data);
                match result {
                    Ok(s) => {

                        return utils::write_string(dest, s).map_err(Error::from)
                        //if self.options.minify {
                            //return utils::write_string_minify(dest, s).map_err(Error::from);
                        //} else {
                            //return utils::write_string(dest, s).map_err(Error::from);
                        //}
                    }
                    Err(e) => return Err(e),
                }
            }
            FileType::Private => {
                // Ignore templates here as they are located and
                // used during the parsing and rendering process
                debug!("noop {}", file.display());
            }
        }

        Ok(())
    }

    pub fn get_invalidation(&mut self, paths: Vec<PathBuf>) -> Result<Invalidation, Error> {
        let mut invalidation = Invalidation{
            layout: false,
            data: false,
            paths: Vec::new()
        };

        let mut src = self.options.source.clone();
        if !src.is_absolute() {
            if let Ok(cwd) = std::env::current_dir() {
                src = cwd.clone();
                src.push(&self.options.source);
            }
        }

        // TODO: handle data.toml files???
        // TODO: handle layout file change - find dependents???
        // TODO: handle partial file changes - find dependents???

        let mut data_file = src.clone();
        data_file.push(DATA_TOML);

        let mut layout_file = src.clone();
        layout_file.push(LAYOUT_HBS);

        for path in paths {
            if path == data_file {
                invalidation.data = true;
            }else if path == layout_file {
                invalidation.layout = true;
            } else {
                if let Some(name) = path.file_name() {
                    let nm = name.to_string_lossy().into_owned();
                    if nm.starts_with(".") {
                        continue;
                    }
                }

                // Prefer relative paths, makes the output much 
                // easier to read
                if let Ok(cwd) = std::env::current_dir() {
                    if let Ok(p) = path.strip_prefix(cwd) {
                        invalidation.paths.push((*p).to_path_buf());
                    } else {
                        invalidation.paths.push(path);
                    }
                } else {
                    invalidation.paths.push(path);
                }
            }
        }

        //println!("invalidation {:?}", invalidation);

        Ok(invalidation)
    }

    pub fn invalidate(&mut self, target: &PathBuf, invalidation: Invalidation) -> Result<(), Error> {
        // FIXME: find out which section of the data.toml changed
        // FIXME: and ensure only those pages are invalidated
        
        // Should we invalidate everything?
        let mut all = false;

        if invalidation.data {
            info!("reload {}", DATA_TOML);
            if let Err(e) = loader::reload(&self.options) {
                error!("{}", e); 
            } else {
                all = true;
            }
        }

        if invalidation.layout {
            all = true;
        }

        if all {
            return self.build(target); 
        } else {
        
            for path in invalidation.paths {
                //println!("process file {:?}", path);
                let file_type = matcher::get_type(&path);
                if let Err(e) = self.process_file(path, file_type) {
                    return Err(e)
                }
            }
        }

        //println!("build files {:?}", invalidation.paths);
        

        Ok(())
    }

    pub fn get_templates_path(&self) -> PathBuf {
        let mut templates = self.options.source.clone();
        templates.push(TEMPLATE);
        templates
    }

    pub fn register_templates_directory(&mut self) -> Result<PathBuf, Error> {
        let templates = self.get_templates_path();
        if let Err(e) = self
            .parser
            .register_templates_directory(TEMPLATE_EXT, templates.as_path())
        {
            return Err(e);
        }
        Ok(templates)
    }

    // Find files and process each entry.
    pub fn build(&mut self, target: &PathBuf) -> Result<(), Error> {
        let templates = self.register_templates_directory()?;

        for result in WalkBuilder::new(&target)
            .follow_links(self.options.follow_links)
            .max_depth(self.options.max_depth)
            .filter_entry(move |e| {
                let path = e.path();

                // Ensure the template directory is ignored
                if path == templates.as_path() {
                    return false;
                }
                true
            })
            .build()
        {
            match result {
                Ok(entry) => {
                    let path = entry.path();

                    // If a file or directory is a descendant of
                    // a book directory we do not process it
                    if self.book.contains_file(&path) {
                        continue;
                    }

                    //println!("ENTRY");
                    if path.is_dir() && self.book.is_book_dir(&path) {
                        // Add the book so we can skip processing of descendants
                        self.book.add(&path);

                        // Build the book
                        if let Err(e) = self.book.build(&path, self.options) {
                            return Err(e);
                        }
                    } else if path.is_file() {
                        //println!("{:?}", entry);

                        let file = entry.path().to_path_buf();
                        let file_type = matcher::get_type(&path);

                        if let Err(e) = self.process_file(file, file_type) {
                            return Err(e)
                        }
                    }
                }
                Err(e) => return Err(Error::IgnoreError(e)),
            }
        }
        Ok(())
    }
}
