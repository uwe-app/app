use std::path::Path;
use std::path::PathBuf;

use config::{FileInfo, FileType};
use datasource::{self, DataSourceMap};

use compiler::context;
use compiler::hook;
use compiler::parser::Parser;
use compiler::Compiler;
use compiler::Error;

/*
 *  Invalidation rules.
 *
 *  - BuildOutput: directory is ignored.
 *  - SiteConfig: (site.toml) is ignored.
 *  - Partial: trigger a build of all pages.
 *  - Layout: trigger a build of all pages.
 *  - Asset: trigger a full build.
 *  - Page: rebuild the page.
 *  - File: copy the file to build.
 *  - Resource: ignored as they are symbolically linked.
 *  - Hook: execute the hook.
 *  - DataSourceConfig: TODO.
 *  - DataSourceDocument: TODO.
 *  - BookTheme: build all books.
 *  - BookConfig: TODO.
 *  - BookSource: build the book.
 */
#[derive(Debug)]
pub enum Action {
    // Because it is valid to configure source = "."
    // in site.toml we need to detect build output and
    // ensure we ignore those files
    BuildOutput(PathBuf),

    // This is not used at the moment but we detect it;
    // it corresponds to the site.toml file.
    SiteConfig(PathBuf),

    Partial(PathBuf),
    Layout(PathBuf),
    Asset(PathBuf),
    Page(PathBuf),
    File(PathBuf),
    Hook(String, PathBuf),
    DataSourceConfig(PathBuf),
    DataSourceDocument(PathBuf),
    // NOTE: The first path is the root directory
    // NOTE: and the second is the matched file.
    BookTheme(PathBuf, PathBuf),
    BookConfig(PathBuf, PathBuf),
    BookSource(PathBuf, PathBuf),
    BookBuild(PathBuf, PathBuf),
}

#[derive(Debug)]
pub enum Strategy {
    // Trigger a full rebuild
    Full,
    // Trigger a build of all pages
    Page,
    // Iterate and process each action
    Mixed,
}

#[derive(Debug)]
pub struct Rule {
    // Notify connected websocket clients, always true for now
    pub notify: bool,
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
pub struct BookRule {
    // Should we build all books
    all: bool,
    // List of books that need their configurations reloaded
    reload: Vec<Action>,
    // List of books that have source file changes and should be built
    source: Vec<Action>,
}

pub struct Invalidator<'a> {
    builder: Compiler<'a>,
    parser: Parser<'a>,
}

impl<'a> Invalidator<'a> {
    pub fn new(builder: Compiler<'a>, parser: Parser<'a>) -> Self {
        Self { builder, parser }
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

    pub fn get_invalidation(&mut self, paths: Vec<PathBuf>) -> Result<Rule, Error> {
        let ctx = self.builder.context;

        //let datasource = &self.builder.context.datasource;

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

        let config_file = &ctx.config.file.as_ref().unwrap();
        let cfg_file = config_file.canonicalize()?;

        let hooks = ctx.config.hook.as_ref().unwrap();

        let build_output = self.canonical(ctx.options.output.clone());

        // NOTE: these files are all optional so we cannot error on
        // NOTE: a call to canonicalize() hence the canonical() helper
        let layout_file = self.canonical(ctx.options.get_layout_path());
        let assets = self.canonical(ctx.options.get_assets_path());
        let partials = self.canonical(ctx.options.get_partials_path());

        // FIXME: this does not respect when data sources have a `from` directory configured
        let generators = self.canonical(ctx.options.get_data_sources_path());

        //let resources = self.canonical(ctx.options.get_resources_path());

        let book_theme = ctx
            .config
            .get_book_theme_path(&ctx.options.source)
            .map(|v| self.canonical(v));

        let mut books: Vec<PathBuf> = Vec::new();
        if let Some(ref book) = ctx.config.book {
            books = book
                .get_paths(&ctx.options.source)
                .iter()
                .map(|p| self.canonical(p))
                .collect::<Vec<_>>();
        }

        //let generator_paths: Vec<PathBuf> = datasource
            //.map
            //.values()
            //.map(|g| self.canonical(g.source.clone()))
            //.collect::<Vec<_>>();

        // TODO: recognise custom layouts (layout = )

        'paths: for path in paths {
            match path.canonicalize() {
                Ok(path) => {
                    // NOTE: must test for hooks first as they can
                    // NOTE: point anywhere in the source directory
                    // NOTE: and should take precedence
                    for (k, hook) in hooks {
                        if hook.source.is_some() {
                            let hook_base =
                                self.canonical(hook.get_source_path(&ctx.options.source).unwrap());
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
                            if let Some(md) = self.builder.book.locate(&ctx.config, &book) {
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
                        //for p in &generator_paths {
                            //let cfg = DataSourceMap::get_datasource_config_path(p);
                            //let documents = datasource::get_datasource_documents_path(p);
                            //if path == cfg {
                                //rule.actions.push(Action::DataSourceConfig(path));
                                //break;
                            //} else if path.starts_with(documents) {
                                //rule.actions.push(Action::DataSourceDocument(path));
                                //break;
                            //}
                        //}
                    } else {
                        let file_type = FileInfo::get_type(&path, &ctx.options.settings);
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

    pub async fn invalidate(&mut self, target: &PathBuf, rule: &Rule) -> Result<(), Error> {
        let ctx = self.builder.context;
        let livereload = context::livereload().read().unwrap();

        let config = &ctx.config;
        let options = &ctx.options;

        // Reload the config data!
        if rule.reload {
            // FIXME: to restore this we need to reload and parse the configuration!
            //if let Err(e) = loader::reload(config, options) {
            //error!("{}", e);
            //}
        }

        for hook in &rule.hooks {
            if let Action::Hook(id, _path) = hook {
                if let Some(hook_config) = config.hook.as_ref().unwrap().get(id) {
                    hook::exec(self.builder.context, hook_config)?;
                }
            }
        }

        let book = &rule.book;

        if !book.reload.is_empty() {
            for action in &book.reload {
                match action {
                    Action::BookConfig(base, _) => {
                        self.builder.book.load(config, base, livereload.clone())?;
                    }
                    _ => {}
                }
            }
        }

        if book.all {
            self.builder.book.all(config, livereload.clone())?;
        } else {
            for action in &book.source {
                match action {
                    Action::BookSource(base, _) => {
                        // Make the path relative to the project source
                        // as the notify crate gives us an absolute path
                        let file = FileInfo::relative_to(base, &options.source, &options.source)?;

                        self.builder.book.build(config, &file, livereload.clone())?;
                    }
                    _ => {}
                }
            }
        }

        match rule.strategy {
            Strategy::Full | Strategy::Page => {
                // TODO: handle updating search index
                let _parse_data = self.builder.build(&self.parser, target).await?;
            }
            _ => {
                for action in &rule.actions {
                    match action {
                        Action::Page(path) | Action::File(path) => {
                            // Make the path relative to the project source
                            // as the notify crate gives us an absolute path
                            let file = FileInfo::relative_to(
                                path,
                                &ctx.options.source,
                                &ctx.options.source,
                            )?;

                            self.builder.one(&self.parser, &file).await?;

                            //if let Err(e) = self.builder.one(&file).await {
                            //return Err(e);
                            //}
                        }
                        _ => {
                            return Err(Error::InvalidationActionNotHandled);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
