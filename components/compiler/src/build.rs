use std::path::Path;
use std::path::PathBuf;

use log::info;

use serde_json::{json, Value};

use book::compiler::BookCompiler;
use config::{Config, Page, FileInfo, FileOptions, ProfileName, IndexQuery};

use crate::{Error, Result, HTML, TEMPLATE_EXT};

use super::context::BuildContext;
use super::hook;
use super::manifest::Manifest;
use super::parser::Parser;
use super::resource;

fn should_minify_html<P: AsRef<Path>>(dest: P, tag: &ProfileName, release: bool, config: &Config) -> bool {
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
    pub context: &'a BuildContext,
    pub book: BookCompiler,
    pub manifest: Manifest<'a>,
    parser: Parser<'a>,
}

impl<'a> Compiler<'a> {
    pub fn new(context: &'a BuildContext) -> Self {
        let book = BookCompiler::new(
            context.options.source.clone(),
            context.options.target.clone(),
            context.options.settings.is_release(),
        );

        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let parser = Parser::new(&context);

        let manifest = Manifest::new(&context);

        Self {
            context,
            book,
            manifest,
            parser,
        }
    }

    fn data_source_each(
        &mut self,
        info: &mut FileInfo,
        data: &Page,
        _reference: IndexQuery,
        values: Vec<Value>,
        rewrite_index: bool,
    ) -> Result<()> {
        let file = info.file;
        let parent = file.parent().unwrap();

        let ctx = self.context;

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

                    let mut file_info = FileInfo::new(
                        &ctx.config,
                        &ctx.options,
                        &mock,
                        true,
                    );

                    let file_opts = FileOptions {
                        rewrite_index,
                        base_href: &ctx.options.settings.base_href,
                        ..Default::default()
                    };

                    file_info.destination(&file_opts)?;
                    let dest = file_info.output.clone().unwrap();

                    // Must inherit the real input template file
                    file_info.file = info.file;

                    info!("{} -> {}", &id, &dest.display());

                    let minify_html = should_minify_html(
                        &dest,
                        &ctx.options.settings.name,
                        ctx.options.settings.is_release(),
                        &ctx.config);

                    let s = if minify_html {
                        minify::html(self.parser.parse(&file_info, &mut item_data)?)
                    } else {
                        self.parser.parse(&file_info, &mut item_data)?
                    };

                    utils::fs::write_string(&dest, &s)?;
                }
            } else {
                return Err(Error::DataSourceDocumentNoId);
            }
        }

        Ok(())
    }

    fn copy_file(&mut self, info: &mut FileInfo) -> Result<()> {
        let file_opts = FileOptions {
            exact: true,
            base_href: &self.context.options.settings.base_href,
            ..Default::default()
        };

        info.destination(&file_opts)?;

        let dest = info.output.as_ref().unwrap();

        let file = info.file;

        if self
            .manifest
            .is_dirty(file, &dest, self.context.options.settings.is_force())
        {
            info!("{} -> {}", file.display(), dest.display());
            utils::fs::copy(file, &dest)?;
            self.manifest.touch(file, &dest);
        } else {
            info!("noop {}", file.display());
        }

        Ok(())
    }

    fn parse_file(&mut self, mut info: &mut FileInfo, mut data: &mut Page) -> Result<()> {
        let file = info.file;

        let ctx = self.context;

        //let mut data = loader::compute(file, &ctx.config, &ctx.options, true)?;

        let render = data.render.is_some() && data.render.unwrap();

        if !render {
            return self.copy_file(info);
        }

        let mut rewrite_index = ctx.options.settings.should_rewrite_index();
        // Override with rewrite-index page level setting
        if let Some(val) = data.rewrite_index {
            rewrite_index = val;
        }

        if super::draft::is_draft(&data, &ctx.options) {
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
                        self.data_source_each(info, &data, gen, idx, rewrite_index)?;
                    }
                    return Ok(());
                }
            }
        }

        let file_opts = FileOptions {
            rewrite_index,
            base_href: &ctx.options.settings.base_href,
            ..Default::default()
        };

        info.destination(&file_opts)?;
        let dest = info.output.clone().unwrap();

        if self
            .manifest
            .is_dirty(file, &dest, ctx.options.settings.is_force())
        {
            info!("{} -> {}", file.display(), dest.display());

            let minify_html = should_minify_html(
                &dest,
                &ctx.options.settings.name,
                ctx.options.settings.is_release(),
                &ctx.config);

            let s = if minify_html {
                minify::html(self.parser.parse(&mut info, &mut data)?)
            } else {
                self.parser.parse(&mut info, &mut data)?
            };

            utils::fs::write_string(&dest, &s)?;
            self.manifest.touch(file, &dest);
        } else {
            info!("noop {}", file.display());
        }

        Ok(())
    }

    pub fn register_templates_directory(&mut self) -> Result<()> {
        let templates = self.context.options.get_partials_path();
        if templates.exists() && templates.is_dir() {
            self.parser.register_templates_directory(TEMPLATE_EXT, &templates)?;
        }
        if self.context.options.settings.should_use_short_codes() {
            let short_codes = self.context.options.get_short_codes_path();
            if short_codes.exists() && short_codes.is_dir() {
                self.parser.register_templates_directory(TEMPLATE_EXT, &short_codes)?;
            }
        }
        Ok(())
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
        let livereload = crate::context::livereload().read().unwrap();

        resource::link(&self.context)?;

        if let Some(hooks) = &self.context.config.hook {
            hook::run(
                self.context,
                hook::collect(
                    hooks.clone(),
                    hook::Phase::Before,
                    &self.context.options.settings.name),
            )?;
        }

        for p in targets {
            if p.is_file() {
                let is_page = self.is_page(&p);
                self.one(&p, is_page)?;
            } else {
                self.build(&p)?;
            }
        }

        // Now compile the books
        if let Some(ref _book) = self.context.config.book {
            self.book
                .all(&self.context.config, livereload.clone())?;
        }

        if let Some(hooks) = &self.context.config.hook {
            hook::run(
                self.context,
                hook::collect(
                    hooks.clone(),
                    hook::Phase::After,
                    &self.context.options.settings.name),
            )?;
        }

        Ok(())
    }

    // Build a single file
    pub fn one(&mut self, file: &PathBuf, is_page: bool) -> Result<()> {
        let mut info = FileInfo::new(
            &self.context.config,
            &self.context.options,
            file,
            false,
        );

        if !is_page {
            self.copy_file(&mut info)?;
        } else {
            let data = self.context.info.all
                .get(file).unwrap().as_ref().unwrap();
            let mut copy = data.clone();
            self.parse_file(&mut info, &mut copy)?;
        }

        Ok(())
    }

    pub fn is_page(&mut self, file: &PathBuf) -> bool {
        self.context.info.pages.contains(&std::sync::Arc::new(file.clone()))
    }

    pub fn build(&mut self, target: &PathBuf) -> Result<()> {
        self.register_templates_directory()?;

        // Files we should copy over
        let copy = self.context.info.other.iter()
            .chain(self.context.info.assets.iter())
            .filter(|p| p.starts_with(target));

        for p in copy {
            self.one(p, false)?;
        }

        let pages = self.context.info.pages
            .iter()
            .filter(|p| p.starts_with(target));

        // Pages to parse
        for p in pages {
            self.one(p, true)?;
        }

        Ok(())
    }
}
