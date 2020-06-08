use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use log::{debug, info, error};

use serde_json::{json, from_value, Map, Value};

pub mod book;
pub mod context;
pub mod generator;
pub mod hook;
pub mod loader;
pub mod helpers;
pub mod manifest;
pub mod matcher;
pub mod parser;
pub mod resource;
pub mod template;

use super::{
    utils,
    Error,
    TEMPLATE_EXT,
    DATA_TOML,
    LAYOUT_HBS
};

use context::Context;
use book::BookBuilder;
use matcher::FileType;
use parser::Parser;
use manifest::Manifest;
use generator::{IndexQuery};

#[derive(Debug)]
pub struct Invalidation {
    data: bool,
    layout: bool,
    paths: Vec<PathBuf>,
}

pub struct Builder<'a> {
    context: &'a Context,
    book: BookBuilder<'a>,
    parser: Parser<'a>,
    manifest: Manifest,
}

impl<'a> Builder<'a> {
    pub fn new(context: &'a Context) -> Self {
        let book = BookBuilder::new(&context);

        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let parser = Parser::new(&context);

        let manifest = Manifest::new();

        Builder {
            context,
            book,
            parser,
            manifest,
        }
    }

    fn each_generator<P: AsRef<Path>>(
        &mut self,
        p: P,
        file_type: &FileType,
        data: &Map<String, Value>,
        _reference: IndexQuery,
        values: Vec<Value>,
        clean: bool) -> Result<(), Error> {

        let file = p.as_ref();
        let parent = file.parent().unwrap();

        // Write out the document files
        for doc in &values {
            let mut item_data = data.clone();

            if let Some(id) = doc.get("id") {
                if let Some(id) = id.as_str() {
                    if doc.is_object() {
                        let map = doc.as_object().unwrap(); 
                        for (k, v) in map {
                            item_data.insert(k.clone(), json!(v));
                        }
                    } else {
                        return Err(Error::new(
                            format!("Generator document should be an object")))
                    }

                    // Mock a source file to build a destination
                    // respecting the clean URL setting
                    let mut mock = parent.to_path_buf();
                    mock.push(&id);
                    if let Some(ext) = file.extension() {
                        mock.set_extension(ext);
                    }

                    let dest = matcher::destination(
                        &self.context.options.source,
                        &self.context.options.target,
                        &mock,
                        &file_type,
                        &self.context.config.extension.as_ref().unwrap(),
                        clean,
                    )?;

                    info!("{} -> {}", &id, &dest.display());

                    //println!("passing item data {:?}", item_data);

                    let s = self.parser.parse(&file, &dest.as_path(), file_type, &mut item_data)?;
                    utils::write_string(&dest, s).map_err(Error::from)?;
                }
            } else {
                return Err(Error::new(format!("Generator document must have an id")))
            }
        }

        Ok(())
    }

    fn process_file<P: AsRef<Path>>(
        &mut self, p: P, file_type: FileType, pages_only: bool) -> Result<(), Error> {

        let file = p.as_ref();
        match file_type {
            FileType::Unknown => {
                let dest = matcher::direct_destination(
                    &self.context.options.source,
                    &self.context.options.target,
                    &file.to_path_buf(),
                )?;

                if self.manifest.is_dirty(file, &dest, self.context.options.force) {
                    info!("{} -> {}", file.display(), dest.display());
                    let result = utils::copy(file, &dest).map_err(Error::from);
                    self.manifest.touch(file, &dest);
                    return result
                } else {
                    info!("noop {}", file.display());
                }
            },
            FileType::Markdown | FileType::Template => {
                let (collides, other) = matcher::collides(file, &file_type);
                if collides {
                    return Err(
                        Error::new(
                            format!("file name collision {} with {}",
                                file.display(),
                                other.display()
                        )))
                }

                let mut data = loader::compute(file);

                let mut clean = self.context.options.clean_url;
                if let Some(val) = data.get("clean") {
                    if let Some(val) = val.as_bool() {
                        clean = val;
                    }
                }

                if utils::is_draft(&data, &self.context.options) {
                    return Ok(())
                }

                let generator_config = data.get("query");
                let mut page_generators: Vec<IndexQuery> = Vec::new();

                if let Some(cfg) = generator_config {
                    // Single object declaration
                    if cfg.is_object() {
                        let conf = cfg.as_object().unwrap();
                        let reference: IndexQuery = from_value(json!(conf))?;
                        page_generators.push(reference);
                    // Multiple array declaration
                    } else if cfg.is_array() {
                        let items = cfg.as_array().unwrap();
                        for conf in items {
                            let reference: IndexQuery = from_value(json!(conf))?;
                            page_generators.push(reference);
                        }
                    } else {
                        return Err(
                            Error::new(
                                format!("Generator parameter should be array or object")));
                    }
                }

                let generators = &self.context.generators;

                if !generators.map.is_empty() {
                
                    let mut each_iters: Vec<(IndexQuery, Vec<Value>)> = Vec::new();

                    for gen in page_generators {
                        let each = gen.each.is_some() && gen.each.unwrap();

                        let idx = generators.query_index(&gen)?;

                        //println!("idx {:?}", idx);

                        // Push on to the list of generators to iterate
                        // over so that we can support the same template
                        // for multiple generator indices although not sure
                        // how useful/desirable it is to declare multiple each iterators
                        // as identifiers may well collide.
                        if each {
                            each_iters.push((gen, idx));
                        } else {
                            data.insert(gen.get_parameter(), json!(idx));
                        }
                    }

                    if !each_iters.is_empty() {
                        for (gen, idx) in each_iters {
                            self.each_generator(&p, &file_type, &data, gen, idx, clean)?;
                        } 
                        return Ok(())
                    }

                }
                
                let dest = matcher::destination(
                    &self.context.options.source,
                    &self.context.options.target,
                    &file.to_path_buf(),
                    &file_type,
                    &self.context.config.extension.as_ref().unwrap(),
                    clean,
                )?;

                if self.manifest.is_dirty(file, &dest, pages_only || self.context.options.force) {
                    info!("{} -> {}", file.display(), dest.display());
                    let s = self.parser.parse(&file, &dest.as_path(), &file_type, &mut data)?;
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

        let mut src = self.context.options.source.clone();
        if !src.is_absolute() {
            if let Ok(cwd) = std::env::current_dir() {
                src = cwd.clone();
                src.push(&self.context.options.source);
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
            if let Err(e) = loader::reload(&self.context.options) {
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
                let file_type = matcher::get_type(&path, &self.context.config.extension.as_ref().unwrap());
                if let Err(e) = self.process_file(&path, file_type, false) {
                    return Err(e)
                }
            }
        }

        //println!("build files {:?}", invalidation.paths);

        Ok(())
    }

    pub fn register_templates_directory(&mut self) -> Result<PathBuf, Error> {
        let templates = self.context.config.get_partial_path(
            &self.context.options.source);

        if let Err(e) = self
            .parser
            .register_templates_directory(TEMPLATE_EXT, templates.as_path())
        {
            return Err(e);
        }
        Ok(templates)
    }

    // Find files and process each entry.
    pub fn build(&mut self, target: &PathBuf, pages_only: bool) -> Result<(), Error> {

        let config_file = self.context.config.file.clone();

        let partials = self.register_templates_directory()?;
        let generator = self.context.config.get_generator_path(
            &self.context.options.source);
        let resource = self.context.config.get_resource_path(
            &self.context.options.source);
        let theme = self.context.config.get_book_theme_path(
            &self.context.options.source);

        let follow_links = self.context.config.build.follow_links.is_some()
            && self.context.config.build.follow_links.unwrap();

        resource::link(self.context)?;

        if let Some(hooks) = &self.context.config.hook {
            hook::run(&self.context, hooks)?;
        }
        
        for result in WalkBuilder::new(&target)
            .follow_links(follow_links)
            .max_depth(self.context.options.max_depth)
            .filter_entry(move |e| {
                let path = e.path();

                if let Some(config_file) = &config_file {
                    if path == config_file.as_path() {
                        return false;
                    }
                }

                if let Some(theme) = &theme {
                    if path == theme.as_path() {
                        return false;
                    }
                }

                if path == partials.as_path()
                    || path == generator.as_path()
                    || path == resource.as_path() {
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
                        if let Err(e) = self.book.build(&path) {
                            return Err(e);
                        }
                    } else if path.is_file() {
                        let file = entry.path().to_path_buf();
                        let file_type = matcher::get_type(&path, &self.context.config.extension.as_ref().unwrap());

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
        let mut file = self.context.options.target.clone();
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
