use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use log::{debug, info, error};

use serde_json::json;

pub mod book;
pub mod generator;
pub mod loader;
pub mod helpers;
pub mod manifest;
pub mod matcher;
pub mod parser;
pub mod template;

use super::{
    utils,
    Error,
    BuildOptions,
    INDEX_HTML,
    TEMPLATE,
    TEMPLATE_EXT,
    DATA_TOML,
    LAYOUT_HBS
};

use book::BookBuilder;
use matcher::FileType;
use parser::Parser;
use manifest::Manifest;

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
    manifest: Manifest,
}

impl<'a> Builder<'a> {
    pub fn new(options: &'a BuildOptions) -> Self {
        let book = BookBuilder::new(options);

        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let parser = Parser::new(options);

        let manifest = Manifest::new();

        Builder {
            options,
            book,
            parser,
            manifest,
        }
    }

    fn process_file<P: AsRef<Path>>(&mut self, p: P, file_type: FileType, pages_only: bool) -> Result<(), Error> {
        let file = p.as_ref();

        match file_type {
            FileType::Unknown => {
                let dest = matcher::direct_destination(
                    &self.options.source,
                    &self.options.target,
                    &file.to_path_buf(),
                )?;

                if self.manifest.is_dirty(file, &dest, self.options.force) {
                    info!("{} -> {}", file.display(), dest.display());
                    let result = utils::copy(file, &dest).map_err(Error::from);
                    self.manifest.touch(file, &dest);
                    return result
                } else {
                    info!("noop {}", file.display());
                }
            },
            FileType::Markdown | FileType::Html => {
                let mut data = loader::compute(file);
                let mut clean = self.options.clean_url;

                if let Some(val) = data.get("clean") {
                    if let Some(val) = val.as_bool() {
                        clean = val;
                    }
                }

                let dest = matcher::destination(
                    &self.options.source,
                    &self.options.target,
                    &file.to_path_buf(),
                    &file_type,
                    clean,
                )?;

                let (collides, other) = matcher::collides(file, &file_type);
                if collides {
                    return Err(
                        Error::new(
                            format!("file name collision {} with {}",
                                file.display(),
                                other.display()
                        )))
                }

                if utils::is_draft(&data, self.options) {
                    return Ok(())
                }

                if self.manifest.is_dirty(file, &dest, pages_only || self.options.force) {
                    info!("{} -> {}", file.display(), dest.display());
                    let s = self.parser.parse(&file, &dest.as_path(), file_type, &mut data)?;
                    let result = utils::write_string(&dest, s).map_err(Error::from);
                    self.manifest.touch(file, &dest);
                    return result
                } else {
                    info!("noop {}", file.display());
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
            return self.build(target, true);
        } else {
        
            for path in invalidation.paths {
                //println!("process file {:?}", path);
                let file_type = matcher::get_type(&path);
                if let Err(e) = self.process_file(&path, file_type, false) {
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

    pub fn build_generators(&mut self) -> Result<(), Error> {
        let generators = generator::GENERATORS.lock().unwrap();
        let clean = self.options.clean_url;

        for (k, g) in generators.iter() {
            let mut tpl = g.source.clone();
            tpl.push(&g.config.build.template);

            let generator_data = loader::compute(k);

            info!("generate {} ({})", k, g.documents.len());

            // Write out the document files
            for doc in &g.documents {
                // Mock a sorce file to build a destination
                // respecting the clean URL setting
                let mut file = self.options.source.clone();
                file.push(&g.config.build.destination);
                file.push(&doc.id);

                let mut data = generator_data.clone();
                data.insert("document".to_string(), json!(&doc.value));

                let file_type = matcher::get_type_extension(&tpl);
                let dest = matcher::destination(
                    &self.options.source,
                    &self.options.target,
                    &file.to_path_buf(),
                    &file_type,
                    clean,
                )?;

                let s = self.parser.parse(&tpl, &dest, file_type, &mut data)?;
                utils::write_string(&dest, s).map_err(Error::from)?;
            }

            // Write out an index page
            if let Some(index_file) = &g.config.build.index {
                let mut index_tpl = g.source.clone();
                index_tpl.push(index_file);

                let mut dest = self.options.target.clone();
                dest.push(&g.config.build.destination);
                dest.push(INDEX_HTML);

                let mut data = generator_data.clone();
                data.insert("documents".to_string(), json!(&g.documents));

                let file_type = matcher::get_type_extension(&index_tpl);
                let s = self.parser.parse(&index_tpl, &dest, file_type, &mut data)?;
                utils::write_string(&dest, s).map_err(Error::from)?;
            }

            // Write out json data
            if let Some(json) = &g.config.build.json {
                let mut file = self.options.source.clone();
                file.push(json);
                if let Ok(s) = serde_json::to_string(&g.documents) {
                    info!("json {}", file.display());
                    utils::write_string(&file, s).map_err(Error::from)?;
                }
            }
        }
        Ok(())
    }

    // Find files and process each entry.
    pub fn build(&mut self, target: &PathBuf, pages_only: bool) -> Result<(), Error> {
        let templates = self.register_templates_directory()?;

        if let Err(e) = self.build_generators() {
            return Err(e)
        }

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

                    if path.is_dir() && self.book.is_book_dir(&path) {
                        // Add the book so we can skip processing of descendants
                        self.book.add(&path);

                        // Build the book
                        if let Err(e) = self.book.build(&path, self.options) {
                            return Err(e);
                        }
                    } else if path.is_file() {
                        let file = entry.path().to_path_buf();
                        let file_type = matcher::get_type(&path);

                        if let Err(e) = self.process_file(&file, file_type, pages_only) {
                            return Err(e)
                        }
                    }
                }
                Err(e) => return Err(Error::IgnoreError(e)),
            }
        }
        Ok(())
    }

    fn get_manifest_file(&self) -> PathBuf {
        let mut file = self.options.target.clone();
        let name = file.file_name().unwrap_or(std::ffi::OsStr::new(""))
            .to_string_lossy().into_owned();
        if !name.is_empty() {
            file.set_file_name(format!("{}.json", name));
        }
        file
    }

    pub fn load_manifest(&mut self) -> Result<(), Error> {
        let file = self.get_manifest_file();
        if file.exists() && file.is_file() {
            debug!("manifest {}", file.display());
            let json = utils::read_string(file)?;
            self.manifest = serde_json::from_str(&json)?;

        }
        Ok(())
    }

    pub fn save_manifest(&self) -> Result<(), Error> {
        let file = self.get_manifest_file();
        let json = serde_json::to_string(&self.manifest)?;
        debug!("manifest {}", file.display());
        utils::write_string(file, json)?;
        Ok(())
    }
}
