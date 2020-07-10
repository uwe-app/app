use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use log::{debug, info};

use serde_json::{json, Value};

use book::compiler::BookCompiler;
use config::{Config, Page, BuildTag, IndexQuery};
use matcher::{self, FileType};

use crate::{Error, Result, HTML, TEMPLATE_EXT};

use super::context::Context;
use super::hook;
use super::manifest::Manifest;
use super::parser::Parser;
use super::resource;

fn should_minify_html<P: AsRef<Path>>(dest: P, tag: &BuildTag, release: bool, config: &Config) -> bool {
    let mut html_extension = false;
    if let Some(ext) = dest.as_ref().extension() {
        html_extension = ext == HTML;
    }

    if html_extension {
        if let Some(ref minify) = config.minify {
            if let Some(ref html) = minify.html {
                if !html.profiles.is_empty() {
                    return html.profiles.contains(tag);
                }
            } 
        }
    }

    release && html_extension
}

pub struct Compiler<'a> {
    context: &'a Context,
    parser: Parser<'a>,
    pub book: BookCompiler,
    pub manifest: Manifest<'a>,
}

impl<'a> Compiler<'a> {
    pub fn new(context: &'a Context) -> Self {
        let book = BookCompiler::new(
            context.options.source.clone(),
            context.options.target.clone(),
            context.options.release,
        );

        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let parser = Parser::new(&context);

        let manifest = Manifest::new(&context);

        Self {
            context,
            parser,
            book,
            manifest,
        }
    }

    fn data_source_each<P: AsRef<Path>>(
        &mut self,
        p: P,
        file_type: &FileType,
        data: &Page,
        _reference: IndexQuery,
        values: Vec<Value>,
        rewrite_index: bool,
    ) -> Result<()> {
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
                            item_data.extra.insert(k.clone(), json!(v));
                        }
                    } else {
                        return Err(Error::DataSourceDocumentNotAnObject);
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
                        rewrite_index,
                        &self.context.options.base_href,
                    )?;

                    info!("{} -> {}", &id, &dest.display());

                    let minify_html = should_minify_html(
                        &dest,
                        &self.context.options.tag,
                        self.context.options.release,
                        &self.context.config);

                    let s = if minify_html {
                        minify::html(
                            self.parser.parse(
                                &file, &dest.as_path(), file_type, &mut item_data)?)
                    } else {
                        self.parser.parse(&file, &dest.as_path(), file_type, &mut item_data)?
                    };

                    utils::fs::write_string(&dest, &s)?;
                }
            } else {
                return Err(Error::DataSourceDocumentNoId);
            }
        }

        Ok(())
    }

    fn copy_file<P: AsRef<Path>>(&mut self, p: P) -> Result<()> {
        let file = p.as_ref();

        let dest = matcher::output(
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
            utils::fs::copy(file, &dest)?;
            self.manifest.touch(file, &dest);
        } else {
            info!("noop {}", file.display());
        }

        Ok(())
    }

    fn parse_file<P: AsRef<Path>>(&mut self, p: P, file_type: FileType) -> Result<()> {
        let file = p.as_ref();

        let mut data = loader::compute(file, &self.context.config, true)?;

        let render = data.render.is_some() && data.render.unwrap();

        if !render {
            return self.copy_file(p)
        }

        let mut rewrite_index = self.context.options.rewrite_index;
        // Override with rewrite-index page level setting
        if let Some(val) = data.rewrite_index {
            rewrite_index = val;
        }

        if super::draft::is_draft(&data, &self.context.options) {
            return Ok(());
        }


        if let Some(ref q) = data.query {
            let queries = q.clone().to_vec();

            let datasource = &self.context.datasource;
            if !datasource.map.is_empty() {
                let mut each_iters: Vec<(IndexQuery, Vec<Value>)> = Vec::new();
                for query in queries {
                    let each = query.each.is_some() && query.each.unwrap();
                    let idx = datasource.query_index(&query)?;

                    // Push on to the list of generators to iterate
                    // over so that we can support the same template
                    // for multiple generator indices although not sure
                    // how useful/desirable it is to declare multiple each iterators
                    // as identifiers may well collide.
                    if each {
                        each_iters.push((query, idx));
                    } else {
                        data.extra.insert(query.get_parameter(), json!(idx));
                    }
                }

                if !each_iters.is_empty() {
                    for (gen, idx) in each_iters {
                        self.data_source_each(&p, &file_type, &data, gen, idx, rewrite_index)?;
                    }
                    return Ok(());
                }
            }
        }

        let dest = matcher::destination(
            &self.context.options.source,
            &self.context.options.target,
            &file.to_path_buf(),
            &file_type,
            &self.context.config.extension.as_ref().unwrap(),
            rewrite_index,
            &self.context.options.base_href,
        )?;

        if self
            .manifest
            .is_dirty(file, &dest, self.context.options.force)
        {
            info!("{} -> {}", file.display(), dest.display());

            let minify_html = should_minify_html(
                &dest,
                &self.context.options.tag,
                self.context.options.release,
                &self.context.config);

            let s = if minify_html {
                minify::html(
                    self.parser.parse(
                        &file, &dest.as_path(), &file_type, &mut data)?)
            } else {
                self.parser.parse(&file, &dest.as_path(), &file_type, &mut data)?
            };

            utils::fs::write_string(&dest, &s)?;
            self.manifest.touch(file, &dest);
        } else {
            info!("noop {}", file.display());
        }

        Ok(())
    }

    fn process_file<P: AsRef<Path>>(&mut self, p: P, file_type: FileType) -> Result<()> {
        match file_type {
            FileType::Unknown => {
                self.copy_file(p)
            }
            FileType::Markdown | FileType::Template => {
                self.parse_file(p, file_type)
            }
        }
    }

    pub fn register_templates_directory(&mut self) -> Result<PathBuf> {
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
    pub fn verify(&self, paths: &Vec<PathBuf>) -> Result<()> {
        for p in paths {
            if !p.starts_with(&self.context.options.source) {
                return Err(Error::OutsideSourceTree(p.clone()));
            }
        }
        Ok(())
    }

    // Build all target paths
    pub fn all(&mut self, targets: Vec<PathBuf>) -> Result<()> {
        for p in targets {
            if p.is_file() {
                self.one(&p)?;
            } else {
                self.build(&p)?;
            }
        }
        Ok(())
    }

    // Build a single file
    pub fn one(&mut self, file: &PathBuf) -> Result<()> {
        let extensions = &self.context.config.extension.as_ref().unwrap();
        let file_type = matcher::get_type(file, extensions);
        self.process_file(file, file_type)
    }

    // Recursively walk and build files in a directory
    pub fn build(&mut self, target: &PathBuf) -> Result<()> {
        self.register_templates_directory()?;

        let build = self.context.config.build.as_ref().unwrap();
        let follow_links = build.follow_links.is_some() && build.follow_links.unwrap();

        let mut filters = matcher::get_filters(
            &self.context.options.source, &self.context.config);

        // Always ignore the layout
        filters.push(self.context.options.layout.clone());

        resource::link(self.context)?;

        if let Some(hooks) = &self.context.config.hook {
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
                    if path.is_file() {
                        let file = path.to_path_buf();
                        self.one(&file)?
                    }
                }
                Err(e) => return Err(Error::from(e)),
            }
        }

        // Now compile the books
        if let Some(ref _book) = self.context.config.book {
            self.book
                .all(&self.context.config, self.context.livereload.clone())?;
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
