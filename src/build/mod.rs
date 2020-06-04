use std::path::Path;
use std::path::PathBuf;
use std::collections::BTreeMap;

use ignore::WalkBuilder;
use log::{debug, info, error};

use serde_json::{json, from_value, Map, Value};

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
    DOCUMENTS,
    JSON,
    GENERATOR,
    TEMPLATE,
    TEMPLATE_EXT,
    DATA_TOML,
    LAYOUT_HBS
};

use book::BookBuilder;
use matcher::FileType;
use parser::Parser;
use manifest::Manifest;
use generator::{Generator, GeneratorReference};

#[derive(Debug)]
pub struct Invalidation {
    data: bool,
    layout: bool,
    paths: Vec<PathBuf>,
}

pub struct Builder<'a> {
    options: &'a BuildOptions,
    generators: &'a BTreeMap<String, Generator>,
    book: BookBuilder<'a>,
    parser: Parser<'a>,
    manifest: Manifest,
}

impl<'a> Builder<'a> {
    pub fn new(options: &'a BuildOptions, generators: &'a BTreeMap<String, Generator>) -> Self {
        let book = BookBuilder::new(options);

        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let parser = Parser::new(options);

        let manifest = Manifest::new();

        Builder {
            options,
            generators,
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
        _reference: GeneratorReference,
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
                        return Err(Error::new(format!("Generator document should be an object")))
                    }

                    // Mock a sorce file to build a destination
                    // respecting the clean URL setting
                    let mut mock = parent.to_path_buf();
                    mock.push(&id);

                    let dest = matcher::destination(
                        &self.options.source,
                        &self.options.target,
                        &mock,
                        &file_type,
                        clean,
                    )?;

                    info!("{} -> {}", &id, &dest.display());

                    let s = self.parser.parse(&file, &dest.as_path(), file_type, &mut item_data)?;
                    utils::write_string(&dest, s).map_err(Error::from)?;
                }
            } else {
                println!("doc {:?}", doc);
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

                let mut clean = self.options.clean_url;
                if let Some(val) = data.get("clean") {
                    if let Some(val) = val.as_bool() {
                        clean = val;
                    }
                }

                if utils::is_draft(&data, self.options) {
                    return Ok(())
                }

                let generator_config = data.get("generator");
                let mut page_generators: Vec<GeneratorReference> = Vec::new();

                if let Some(cfg) = generator_config {
                    // Single object declaration
                    if cfg.is_object() {
                        let conf = cfg.as_object().unwrap();
                        let reference: GeneratorReference = from_value(json!(conf))?;
                        page_generators.push(reference);
                    // Multiple array declaration
                    } else if cfg.is_array() {
                        let items = cfg.as_array().unwrap();
                        for conf in items {
                            let reference: GeneratorReference = from_value(json!(conf))?;
                            page_generators.push(reference);
                        }
                    } else {
                        return Err(
                            Error::new(
                                format!("Generator parameter should be array or object")));
                    }
                }

                let mut each_iters: Vec<(GeneratorReference, Vec<Value>)> = Vec::new();

                for gen in page_generators {
                    let each = gen.each.is_some() && gen.each.unwrap();

                    let idx = generator::find_generator_index(self.generators, &gen)?;
                    if let Some(key) = &gen.parameter {
                        data.insert(key.clone(), json!(idx));
                    }

                    // Push on to the list of generators to iterate
                    // over so that we can support the same template
                    // for multiple generator indices although not sure
                    // how useful/desirable it is to declare multiple each iterators
                    // as identifiers may well collide.
                    if each {
                        each_iters.push((gen, idx));
                    }
                }

                if !each_iters.is_empty() {
                    for (gen, idx) in each_iters {
                        self.each_generator(&p, &file_type, &data, gen, idx, clean)?;
                    } 
                    return Ok(())
                }

                let dest = matcher::destination(
                    &self.options.source,
                    &self.options.target,
                    &file.to_path_buf(),
                    &file_type,
                    clean,
                )?;

                if self.manifest.is_dirty(file, &dest, pages_only || self.options.force) {
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
        for (k, g) in self.generators.iter() {
            let all = &g.all;
            info!("generate {} ({})", k, all.documents.len());

            // Copy over the JSON documents when asked
            if let Some(json) = &g.config.json {

                if json.copy {
                    // Write out the document files
                    for doc in &all.documents {
                        let mut file = g.source.clone();
                        file.push(DOCUMENTS);
                        file.push(&doc.id);
                        file.set_extension(JSON);

                        let mut dest = self.options.target.clone();
                        dest.push(&g.config.build.destination);
                        dest.push(&doc.id);
                        dest.set_extension(JSON);
                        debug!("{} -> {}", &file.display(), &dest.display());
                        utils::copy(&file, &dest).map_err(Error::from)?;
                    }
                }

                // Write out json index
                if let Some(file_name) = &json.index_file {
                    let mut file = self.options.target.clone();
                    file.push(&g.config.build.destination);
                    file.push(file_name);

                    // Just write out the identifiers
                    if json.index_slim {
                        let list: Vec<&String> = all.documents
                            .iter()
                            .map(|d| &d.id)
                            .collect::<Vec<_>>();
                        if let Ok(s) = serde_json::to_string(&list) {
                            info!("json {}", file.display());
                            utils::write_string(&file, s).map_err(Error::from)?;
                        }
                    // Write out identifiers with the document values
                    } else {
                        if let Ok(s) = serde_json::to_string(&all.documents) {
                            info!("json {}", file.display());
                            utils::write_string(&file, s).map_err(Error::from)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // Find files and process each entry.
    pub fn build(&mut self, target: &PathBuf, pages_only: bool) -> Result<(), Error> {
        let templates = self.register_templates_directory()?;

        let mut generator = self.options.source.to_path_buf();
        generator.push(GENERATOR);
        

        if let Err(e) = self.build_generators() {
            return Err(e)
        }

        for result in WalkBuilder::new(&target)
            .follow_links(self.options.follow_links)
            .max_depth(self.options.max_depth)
            .filter_entry(move |e| {
                let path = e.path();

                // Ensure the template directory is ignored
                if path == templates.as_path() || path == generator.as_path() {
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
