use std::path::PathBuf;

use super::Builder;
use super::context::Context;
use super::loader;
use super::matcher;
use super::generator;
use super::matcher::FileType;

use tokio::sync::broadcast::Sender;
use warp::ws::Message;

use crate::{
    Error,
    DATA_TOML,
    LAYOUT_HBS
};

use log::{info, error};

use super::watch;

use crate::callback::ErrorCallback;

/*
 *  Invalidation rules.
 *
 *  - BuildOutput: directory is ignored.
 *  - SiteConfig: (site.toml) is ignored.
 *  - DataConfig: (data.toml) trigger a build of all pages.
 *  - Partial: trigger a build of all pages.
 *  - Layout: trigger a build of all pages.
 *  - Asset: trigger a full build.
 *  - Page: rebuild the page.
 *  - File: copy the file to build.
 *  - Resource: ignored as they are symbolically linked.
 *  - Hook: execute the hook.
 *  - GeneratorConfig: TODO.
 *  - GeneratorDocument: TODO.
 *  - BookTheme: build all books.
 *  - BookConfig: TODO.
 *  - BookSource: build the book.
 */
#[derive(Debug)]
pub enum InvalidationType {
    // Because it is valid to configure source = "."
    // in site.toml we need to detect build output and
    // ensure we ignore those files
    BuildOutput(PathBuf),

    // This is not used at the moment but we detect it;
    // it corresponds to the site.toml file.
    SiteConfig(PathBuf),

    DataConfig(PathBuf),
    Partial(PathBuf),
    Layout(PathBuf),
    Asset(PathBuf),
    Page(PathBuf),
    File(PathBuf),
    Resource(PathBuf),
    Hook(String, PathBuf),
    GeneratorConfig(PathBuf),
    GeneratorDocument(PathBuf),
    // NOTE: The first path is the root directory
    // NOTE: and the second is the matched file.
    BookTheme(PathBuf, PathBuf),
    BookConfig(PathBuf, PathBuf),
    BookSource(PathBuf, PathBuf),
}

#[derive(Debug)]
pub enum BuildStrategy {
    Full,
    Pages,
    Single,
    Mixed,
}

#[derive(Debug)]
pub struct InvalidationRule {
    reload: bool,
    strategy: BuildStrategy,
    actions: Vec<InvalidationType>,
}

//#[derive(Debug)]
//pub struct Invalidation {
    //data: bool,
    //layout: bool,
    //paths: Vec<PathBuf>,
//}

pub struct Invalidator<'a> {
    context: &'a Context,
    builder: Builder<'a>,
}

impl<'a> Invalidator<'a> {
    pub fn new(context: &'a Context, builder: Builder<'a>) -> Self {
        Self { context, builder }
    }

    pub fn start(&mut self, from: PathBuf, tx: Sender<Message>, error_cb: &ErrorCallback) -> Result<(), Error> {
        #[cfg(feature = "watch")]
        let watch_result = watch::start(&from.clone(), error_cb, move |paths, source_dir| {
            info!("changed({}) in {}", paths.len(), source_dir.display());

            match self.get_invalidation(paths) {
                Ok(invalidation) => {
                    match self.invalidate(&from, &invalidation) {
                        Ok(_) => {
                            self.builder.save_manifest()?;
                            if invalidation.reload {
                                let _ = tx.send(Message::text("reload"));
                            }
                            Ok(())
                        },
                        Err(e) => {
                            return Err(e)
                        },
                    }
                },
                Err(e) => {
                    return Err(e)
                }
            }
        });

        if let Err(e) = watch_result {
            return Err(e)
        }

        Ok(())
    }

    fn canonical(&mut self, src: PathBuf) -> PathBuf {
        if src.exists() {
            if let Ok(canonical) = src.canonicalize() {
                return canonical;
            }
        }
        src
    }

    fn get_invalidation(&mut self, paths: Vec<PathBuf>) -> Result<InvalidationRule, Error> {

        let mut rule = InvalidationRule{
            reload: true,
            strategy: BuildStrategy::Mixed,
            actions: Vec::new(),
        };

        let config_file = &self.context.config.file.as_ref().unwrap();
        let cfg_file = config_file.canonicalize()?;

        let hooks = self.context.config.hook.as_ref().unwrap();

        let build_output = self.canonical(self.context.options.output.clone());

        // NOTE: these files are all optional so we cannot error on
        // NOTE: a call to canonicalize() hence the canonical() helper
        let data_file = self.canonical(
            self.context.config.get_data_path(
                &self.context.options.source));

        let layout_file = self.canonical(
            self.context.config.get_layout_path(
                &self.context.options.source));

        let assets = self.canonical(
            self.context.config.get_assets_path(
                &self.context.options.source));

        let partials = self.canonical(
            self.context.config.get_partials_path(
                &self.context.options.source));

        let generators = self.canonical(
            self.context.config.get_generators_path(
                &self.context.options.source));

        let resources = self.canonical(
            self.context.config.get_resources_path(
                &self.context.options.source));

        let book_theme = self.context.config.get_book_theme_path(
            &self.context.options.source).map(|v| self.canonical(v));

        let books: Vec<PathBuf> = self.builder.book.books
            .clone()
            .iter()
            .map(|p| self.canonical(p.to_path_buf()))
            .collect::<Vec<_>>();

        let generator_paths: Vec<PathBuf> = self.context.generators.map
            .values()
            .map(|g| self.canonical(g.source.clone()))
            .collect::<Vec<_>>();

        // TODO: recognise custom layouts (layout = )

        'paths: for path in paths {
            match path.canonicalize() {
                Ok(path) => {

                    // NOTE: must test for hooks first as they can
                    // NOTE: point anywhere in the source directory
                    // NOTE: and should take precedence
                    for (k, hook) in hooks {
                        if hook.source.is_some() {
                            let hook_base = self.canonical(
                                hook.get_source_path(
                                    &self.context.options.source).unwrap());
                            if path.starts_with(hook_base) {
                                rule.actions.push(
                                    InvalidationType::Hook(k.clone(), path));
                                continue 'paths;
                            }
                        }
                    }

                    for book_path in &books {
                        let cfg = self.builder.book.get_book_config(book_path);
                        if path == cfg {
                            rule.actions.push(InvalidationType::BookConfig(book_path.clone(), path));
                            continue 'paths;
                        }
                        if path.starts_with(book_path) {
                            if let Some(book) = self.builder.book.references.get(book_path) {
                                let src = &book.config.book.src;
                                let mut buf = book_path.clone();
                                buf.push(src);
                                if path.starts_with(buf) {
                                    rule.actions.push(
                                        InvalidationType::BookSource(book_path.clone(), path));
                                    continue 'paths;
                                }

                            }
                        }
                    }

                    if let Some(theme) = &book_theme {
                        if path.starts_with(theme) {
                            rule.actions.push(
                                InvalidationType::BookTheme(theme.clone(), path));
                            continue 'paths;
                        }
                    }

                    if path == cfg_file {
                        rule.actions.push(InvalidationType::SiteConfig(path));
                    } else if path == data_file {
                        rule.actions.push(InvalidationType::DataConfig(path));
                    } else if path == layout_file {
                        rule.actions.push(InvalidationType::Layout(path));
                    } else if path.starts_with(&build_output) {
                        rule.actions.push(InvalidationType::BuildOutput(path));
                    } else if path.starts_with(&assets) {
                        rule.actions.push(InvalidationType::Asset(path));
                    } else if path.starts_with(&partials) {
                        rule.actions.push(InvalidationType::Partial(path));
                    } else if path.starts_with(&generators) {
                        for p in &generator_paths {
                            let cfg = self.context.generators.get_generator_config_path(p);
                            let documents = generator::get_generator_documents_path(p);
                            if path == cfg {
                                rule.actions.push(InvalidationType::GeneratorConfig(path));
                                break;
                            } else if path.starts_with(documents) {
                                rule.actions.push(InvalidationType::GeneratorDocument(path));
                                break;
                            }
                        }
                    } else if path.starts_with(&resources) {
                        rule.actions.push(InvalidationType::Resource(path));
                    } else {
                        let extensions = &self.context.config.extension.as_ref().unwrap();
                        let file_type = matcher::get_type_extension(&path, extensions);
                        match file_type {
                            FileType::Unknown => {
                                rule.actions.push(InvalidationType::File(path));
                            },
                            _ => {
                                rule.actions.push(InvalidationType::Page(path));
                            }
                        }
                    }
                },
                Err(e) => return Err(Error::from(e)),
            }
        }
        Ok(rule)
    }

    /*
    fn get_invalidation(&mut self, paths: Vec<PathBuf>) -> Result<Invalidation, Error> {
        let types = self.get_path_types(&paths)?;
        println!("Types {:?}", types);

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
                    //if let Ok(p) = path.strip_prefix(cwd) {
                        //invalidation.paths.push((*p).to_path_buf());
                    //} else {
                        //invalidation.paths.push(path);
                    //}
                } else {
                    invalidation.paths.push(path);
                }
            }
        }

        Ok(invalidation)
    }
    */

    fn invalidate(&mut self, target: &PathBuf, rule: &InvalidationRule) -> Result<(), Error> {

        println!("do the invalidation");

        // FIXME: find out which section of the data.toml changed
        // FIXME: and ensure only those pages are invalidated

        // Should we invalidate everything?
        //let mut all = false;

        //if invalidation.data {
            //info!("reload {}", DATA_TOML);
            //if let Err(e) = loader::reload(&self.context.config, &self.context.options.source) {
                //error!("{}", e);
            //} else {
                //all = true;
            //}
        //}

        //if invalidation.layout {
            //all = true;
        //}

        //if all {
            //println!("build all");
            //return self.builder.build(target, true);
        //} else {

            //for path in invalidation.paths {
                //let file_type = matcher::get_type(&path, &self.context.config.extension.as_ref().unwrap());
                //println!("build one");
                //if let Err(e) = self.builder.process_file(&path, file_type, false) {
                    //return Err(e)
                //}
            //}
        //}
        Ok(())
    }
}
