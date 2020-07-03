use std::path::Path;
use std::path::PathBuf;

use log::{error, info};
use tokio::sync::broadcast::Sender;
use warp::ws::Message;

use super::context::Context;
use super::generator;
use super::hook;
use super::loader;
use super::matcher;
use super::matcher::FileType;
use super::watch;
use super::compiler::Compiler;

use crate::ErrorCallback;
use crate::Error;

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
enum Action {
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
    BookBuild(PathBuf, PathBuf),
}

#[derive(Debug)]
enum Strategy {
    // Trigger a full rebuild
    Full,
    // Trigger a build of all pages
    Page,
    // Iterate and process each action
    Mixed,
}

#[derive(Debug)]
struct Rule {
    // Notify connected websocket clients, always true for now
    notify: bool,
    // Reload the site data source
    reload: bool,
    // Build strategy
    strategy: Strategy,
    // Books have their own rules
    book: BookRule,
    // Actions that are ignored but we track for debugging
    ignores: Vec<Action>,
    // Hooks are a special case so we store them separately
    hooks: Vec<Action>,
    // List of actions corresponding to the files that changed
    actions: Vec<Action>,
}

#[derive(Debug)]
struct BookRule {
    // Should we build all books
    all: bool,
    // List of books that need their configurations reloaded
    reload: Vec<Action>,
    // List of books that have source file changes and should be built
    source: Vec<Action>,
}

pub struct Invalidator<'a> {
    context: &'a Context,
    builder: Compiler<'a>,
}

impl<'a> Invalidator<'a> {
    pub fn new(context: &'a Context, builder: Compiler<'a>) -> Self {
        Self { context, builder }
    }

    pub fn start(
        &mut self,
        from: PathBuf,
        tx: Sender<Message>,
        error_cb: &ErrorCallback,
    ) -> Result<(), Error> {
        let watch_result = watch::start(&from.clone(), error_cb, move |paths, source_dir| {
            info!("changed({}) in {}", paths.len(), source_dir.display());

            match self.get_invalidation(paths) {
                Ok(invalidation) => match self.invalidate(&from, &invalidation) {
                    Ok(_) => {
                        self.builder.manifest.save()?;
                        if invalidation.notify {
                            let _ = tx.send(Message::text("reload"));
                        }
                        Ok(())
                    }
                    Err(e) => return Err(e),
                },
                Err(e) => return Err(e),
            }
        });

        if let Err(e) = watch_result {
            return Err(e);
        }

        Ok(())
    }

    fn canonical<P: AsRef<Path>>(&mut self, src: P) -> PathBuf {
        let file = src.as_ref().to_path_buf();
        if file.exists() {
            if let Ok(canonical) = file.canonicalize() {
                return canonical;
            }
        }
        file
    }

    fn get_invalidation(&mut self, paths: Vec<PathBuf>) -> Result<Rule, Error> {
        let mut rule = Rule {
            notify: true,
            reload: false,
            strategy: Strategy::Mixed,
            ignores: Vec::new(),
            hooks: Vec::new(),
            book: BookRule {
                all: false,
                reload: Vec::new(),
                source: Vec::new(),
            },
            actions: Vec::new(),
        };

        let config_file = &self.context.config.file.as_ref().unwrap();
        let cfg_file = config_file.canonicalize()?;

        let hooks = self.context.config.hook.as_ref().unwrap();

        let build_output = self.canonical(self.context.options.output.clone());

        // NOTE: these files are all optional so we cannot error on
        // NOTE: a call to canonicalize() hence the canonical() helper
        let data_file = self.canonical(self.context.config.get_page_data_path());

        let layout_file = self.canonical(
            self.context
                .config
                .get_layout_path(&self.context.options.source),
        );

        let assets = self.canonical(
            self.context
                .config
                .get_assets_path(&self.context.options.source),
        );

        let partials = self.canonical(
            self.context
                .config
                .get_partials_path(&self.context.options.source),
        );

        let generators = self.canonical(
            self.context
                .config
                .get_datasources_path(&self.context.options.source),
        );

        let resources = self.canonical(
            self.context
                .config
                .get_resources_path(&self.context.options.source),
        );

        let book_theme = self
            .context
            .config
            .get_book_theme_path(&self.context.options.source)
            .map(|v| self.canonical(v));

        let mut books: Vec<PathBuf> = Vec::new();
        if let Some(ref book) = self.context.config.book {
            books = book.get_paths(&self.context.options.source);
        }

        let generator_paths: Vec<PathBuf> = self
            .context
            .generators
            .map
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
                                hook.get_source_path(&self.context.options.source).unwrap(),
                            );
                            if path.starts_with(hook_base) {
                                rule.hooks.push(Action::Hook(k.clone(), path));
                                continue 'paths;
                            }
                        }
                    }

                    for book_path in &books {
                        let book = self.canonical(book_path);

                        let cfg = self.builder.book.get_book_config(&book);
                        if path == cfg {
                            rule.book
                                .reload
                                .push(Action::BookConfig(book.clone(), path));
                            continue 'paths;
                        }
                        if path.starts_with(book_path) {
                            if let Some(md) = self.builder.book.locate(&self.context.config, &book) {
                                let src_dir = &md.config.book.src;
                                let build_dir = &md.config.build.build_dir;

                                let mut src = book.clone();
                                src.push(src_dir);

                                let mut build = book.clone();
                                build.push(build_dir);

                                if path.starts_with(build) {
                                    rule.ignores.push(Action::BookBuild(book.clone(), path));
                                    continue 'paths;
                                } else if path.starts_with(src) {
                                    rule.book
                                        .source
                                        .push(Action::BookSource(book.clone(), path));
                                    continue 'paths;
                                }
                            }
                        }
                    }

                    if let Some(theme) = &book_theme {
                        if path.starts_with(theme) {
                            rule.book.all = true;
                            rule.ignores.push(Action::BookTheme(theme.clone(), path));
                            continue 'paths;
                        }
                    }

                    if path == cfg_file {
                        rule.ignores.push(Action::SiteConfig(path));
                    } else if path == data_file {
                        // FIXME: find out which section of the data.toml changed
                        // FIXME: and ensure only those pages are invalidated
                        rule.reload = true;
                        rule.strategy = Strategy::Page;
                        rule.ignores.push(Action::DataConfig(path));
                    } else if path == layout_file {
                        rule.strategy = Strategy::Page;
                        rule.ignores.push(Action::Layout(path));
                    } else if path.starts_with(&build_output) {
                        rule.ignores.push(Action::BuildOutput(path));
                    } else if path.starts_with(&assets) {
                        rule.strategy = Strategy::Full;
                        rule.actions.push(Action::Asset(path));
                    } else if path.starts_with(&partials) {
                        rule.strategy = Strategy::Page;
                        rule.ignores.push(Action::Partial(path));
                    } else if path.starts_with(&generators) {
                        for p in &generator_paths {
                            let cfg = self.context.generators.get_generator_config_path(p);
                            let documents = generator::get_generator_documents_path(p);
                            if path == cfg {
                                rule.actions.push(Action::GeneratorConfig(path));
                                break;
                            } else if path.starts_with(documents) {
                                rule.actions.push(Action::GeneratorDocument(path));
                                break;
                            }
                        }
                    } else if path.starts_with(&resources) {
                        rule.ignores.push(Action::Resource(path));
                    } else {
                        let extensions = &self.context.config.extension.as_ref().unwrap();
                        let file_type = matcher::get_type(&path, extensions);
                        match file_type {
                            FileType::Unknown => {
                                rule.actions.push(Action::File(path));
                            }
                            _ => {
                                rule.actions.push(Action::Page(path));
                            }
                        }
                    }
                }
                Err(e) => return Err(Error::from(e)),
            }
        }

        // This is a fix for double location.reload on books,
        // the `book` build directory is also watched which
        // would generate a lot of ignores and trigger a
        // second websocket notification, this check disables it.
        //
        // Once the logic for selecting watch directories is implemented
        // this can probably be removed.
        let is_empty =
            rule.actions.is_empty() && rule.hooks.is_empty() && rule.book.source.is_empty();
        match rule.strategy {
            Strategy::Mixed => {
                if is_empty {
                    rule.notify = false;
                }
            }
            _ => {}
        }

        Ok(rule)
    }

    fn invalidate(&mut self, target: &PathBuf, rule: &Rule) -> Result<(), Error> {
        // Reload the data source
        if rule.reload {
            if let Err(e) = loader::reload(&self.context.config, &self.context.options.source) {
                error!("{}", e);
            }
        }

        for hook in &rule.hooks {
            if let Action::Hook(id, _path) = hook {
                if let Some(hook_config) = &self.context.config.hook.as_ref().unwrap().get(id) {
                    hook::exec(&self.context, hook_config)?;
                }
            }
        }

        let book = &rule.book;

        if !book.reload.is_empty() {
            for action in &book.reload {
                match action {
                    Action::BookConfig(base, _) => {
                        self.builder.book.load(
                            &self.context.config,
                            base,
                            self.context.livereload.clone())?;
                    }
                    _ => {}
                }
            }
        }

        if book.all {
            self.builder.book.all(
                &self.context.config,
                self.context.livereload.clone())?;
        } else {
            for action in &book.source {
                match action {
                    Action::BookSource(base, _) => {
                        // Make the path relative to the project source
                        // as the notify crate gives us an absolute path
                        let file = matcher::relative_to(
                            base,
                            &self.context.options.source,
                            &self.context.options.source,
                        )?;

                        self.builder.book.build(
                            &self.context.config,
                            &file,
                            self.context.livereload.clone())?;
                    }
                    _ => {}
                }
            }
        }

        match rule.strategy {
            Strategy::Full => {
                return self.builder.build(target);
            }
            Strategy::Page => {
                return self.builder.build(target);
            }
            _ => {
                for action in &rule.actions {
                    match action {
                        Action::Page(path) | Action::File(path) => {
                            // Make the path relative to the project source
                            // as the notify crate gives us an absolute path
                            let file = matcher::relative_to(
                                path,
                                &self.context.options.source,
                                &self.context.options.source,
                            )?;

                            if let Err(e) = self.builder.one(&file) {
                                return Err(e);
                            }
                        }
                        _ => {
                            return Err(Error::new("Invalidation action not handled".to_string()));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
