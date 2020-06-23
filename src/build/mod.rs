use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use log::{debug, info};

use serde_json::{json, Value};

pub mod book;
pub mod context;
pub mod frontmatter;
pub mod generator;
pub mod helpers;
pub mod hook;
pub mod invalidator;
pub mod loader;
pub mod manifest;
pub mod matcher;
pub mod page;
pub mod parser;
pub mod redirect;
pub mod resource;
pub mod template;
pub mod tree;
pub mod watch;

use crate::{utils, Error, TEMPLATE_EXT};

use book::BookBuilder;
use context::Context;
use generator::IndexQuery;
use manifest::Manifest;
use matcher::FileType;
use page::Page;
use parser::Parser;

pub struct Builder<'a> {
    context: &'a Context,
    book: BookBuilder<'a>,
    parser: Parser<'a>,
    pub manifest: Manifest<'a>,
}

impl<'a> Builder<'a> {
    pub fn new(context: &'a Context) -> Self {
        let book = BookBuilder::new(&context);

        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let parser = Parser::new(&context);

        let manifest = Manifest::new(&context);

        Self {
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
        data: &Page,
        _reference: IndexQuery,
        values: Vec<Value>,
        clean: bool,
    ) -> Result<(), Error> {
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
                            item_data.vars.insert(k.clone(), json!(v));
                        }
                    } else {
                        return Err(Error::new(format!(
                            "Generator document should be an object"
                        )));
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
                        &self.context.options.base_href,
                    )?;

                    info!("{} -> {}", &id, &dest.display());

                    let s = self
                        .parser
                        .parse(&file, &dest.as_path(), file_type, &mut item_data)?;
                    utils::write_string(&dest, s).map_err(Error::from)?;
                }
            } else {
                return Err(Error::new(format!("Generator document must have an id")));
            }
        }

        Ok(())
    }

    pub fn process_file<P: AsRef<Path>>(
        &mut self,
        p: P,
        file_type: FileType,
        pages_only: bool,
    ) -> Result<(), Error> {
        let file = p.as_ref();
        match file_type {
            FileType::Unknown => {
                let dest = matcher::direct_destination(
                    &self.context.options.source,
                    &self.context.options.target,
                    &file.to_path_buf(),
                    &self.context.options.base_href,
                )?;

                if self
                    .manifest
                    .is_dirty(file, &dest, self.context.options.force)
                {
                    info!("{} -> {}", file.display(), dest.display());
                    let result = utils::copy(file, &dest).map_err(Error::from);
                    self.manifest.touch(file, &dest);
                    return result;
                } else {
                    info!("noop {}", file.display());
                }
            }
            FileType::Markdown | FileType::Template => {
                let (collides, other) = matcher::collides(file, &file_type);
                if collides {
                    return Err(Error::new(format!(
                        "file name collision {} with {}",
                        file.display(),
                        other.display()
                    )));
                }

                let mut data = loader::compute(file, &self.context.config, true)?;

                let mut clean = self.context.options.clean_url;
                if let Some(val) = data.clean {
                    clean = val;
                }

                if utils::is_draft(&data, &self.context.options) {
                    return Ok(());
                }

                let queries = generator::get_query(&data)?;

                let generators = &self.context.generators;

                if !generators.map.is_empty() {
                    let mut each_iters: Vec<(IndexQuery, Vec<Value>)> = Vec::new();
                    for query in queries {
                        let each = query.each.is_some() && query.each.unwrap();
                        let idx = generators.query_index(&query)?;

                        // Push on to the list of generators to iterate
                        // over so that we can support the same template
                        // for multiple generator indices although not sure
                        // how useful/desirable it is to declare multiple each iterators
                        // as identifiers may well collide.
                        if each {
                            each_iters.push((query, idx));
                        } else {
                            data.vars.insert(query.get_parameter(), json!(idx));
                        }
                    }

                    if !each_iters.is_empty() {
                        for (gen, idx) in each_iters {
                            self.each_generator(&p, &file_type, &data, gen, idx, clean)?;
                        }
                        return Ok(());
                    }
                }

                let dest = matcher::destination(
                    &self.context.options.source,
                    &self.context.options.target,
                    &file.to_path_buf(),
                    &file_type,
                    &self.context.config.extension.as_ref().unwrap(),
                    clean,
                    &self.context.options.base_href,
                )?;

                if self
                    .manifest
                    .is_dirty(file, &dest, pages_only || self.context.options.force)
                {
                    info!("{} -> {}", file.display(), dest.display());
                    let s = self
                        .parser
                        .parse(&file, &dest.as_path(), &file_type, &mut data)?;
                    let result = utils::write_string(&dest, s).map_err(Error::from);
                    self.manifest.touch(file, &dest);
                    return result;
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

    pub fn register_templates_directory(&mut self) -> Result<PathBuf, Error> {
        let templates = self
            .context
            .config
            .get_partials_path(&self.context.options.source);

        if let Err(e) = self
            .parser
            .register_templates_directory(TEMPLATE_EXT, templates.as_path())
        {
            return Err(e);
        }
        Ok(templates)
    }

    // Verify the paths are within the site source
    pub fn verify(&self, paths: &Vec<PathBuf>) -> Result<(), Error> {
        for p in paths {
            if !p.starts_with(&self.context.options.source) {
                return Err(Error::new(format!(
                    "Path '{}' is outside the site source",
                    p.display()
                )));
            }
        }
        Ok(())
    }

    // Build all target paths
    pub fn all(&mut self, targets: Vec<PathBuf>, pages_only: bool) -> Result<(), Error> {
        for p in targets {
            if p.is_file() {
                self.one(&p, pages_only)?;
            } else {
                self.build(&p, pages_only)?;
            }
        }
        Ok(())
    }

    // Build a single file
    pub fn one(&mut self, file: &PathBuf, pages_only: bool) -> Result<(), Error> {
        let extensions = &self.context.config.extension.as_ref().unwrap();
        let file_type = matcher::get_type(file, extensions);
        self.process_file(file, file_type, pages_only)
    }

    // Recursively walk and build files in a directory
    pub fn build(&mut self, target: &PathBuf, pages_only: bool) -> Result<(), Error> {
        let config_file = self.context.config.file.clone();

        let partials = self.register_templates_directory()?;
        let generator = self
            .context
            .config
            .get_generators_path(&self.context.options.source);
        let resource = self
            .context
            .config
            .get_resources_path(&self.context.options.source);
        let theme = self
            .context
            .config
            .get_book_theme_path(&self.context.options.source);

        let build = self.context.config.build.as_ref().unwrap();
        let follow_links = build.follow_links.is_some() && build.follow_links.unwrap();

        let mut filters: Vec<PathBuf> = Vec::new();
        filters.push(partials);
        filters.push(generator);
        filters.push(resource);

        if let Some(config_file) = &config_file {
            filters.push(config_file.clone());
        }

        if let Some(theme) = &theme {
            filters.push(theme.clone());
        }

        if let Some(locales_dir) = self
            .context
            .config
            .get_locales(&self.context.options.source)
        {
            filters.push(locales_dir);
        }

        resource::link(self.context)?;

        if let Some(hooks) = &self.context.config.hook {
            for (_, v) in hooks {
                if let Some(source) = &v.source {
                    let mut buf = self.context.options.source.clone();
                    buf.push(source);
                    filters.push(buf);
                }
            }
            hook::run(
                &self.context,
                hook::collect(hooks.clone(), hook::Phase::Before),
            )?;
        }

        for result in WalkBuilder::new(&target)
            .follow_links(follow_links)
            .max_depth(self.context.options.max_depth)
            .filter_entry(move |e| {
                let path = e.path();
                if filters.contains(&path.to_path_buf()) {
                    debug!("SKIP {}", path.display());
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
                        //self.book.add(&path);

                        // Build the book
                        self.book.load(&self.context, &path)?;
                        self.book.build(&path)?;
                    } else if path.is_file() {
                        let file = path.to_path_buf();
                        self.one(&file, pages_only)?
                    }
                }
                Err(e) => return Err(Error::IgnoreError(e)),
            }
        }

        if let Some(hooks) = &self.context.config.hook {
            hook::run(
                &self.context,
                hook::collect(hooks.clone(), hook::Phase::After),
            )?;
        }

        Ok(())
    }
}
