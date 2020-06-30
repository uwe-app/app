use std::collections::BTreeMap;
use std::convert::AsRef;
use std::path::Path;
use std::path::PathBuf;

use ignore::WalkBuilder;
use log::{debug, info, warn};
use mdbook::MDBook;

use crate::build::loader;

static BOOK_TOML: &str = "book.toml";
static BOOK_THEME_KEY: &str = "output.html.theme";

use crate::{utils, Error, DRAFT_KEY};

use super::context::Context;

static BOOK_SITE_TABLE_KEY: &str = "site";

pub struct BookBuilder<'a> {
    //pub books: Vec<PathBuf>,
    pub references: BTreeMap<PathBuf, MDBook>,
    context: &'a Context,
}

impl<'a> BookBuilder<'a> {
    pub fn new(context: &'a Context) -> Self {
        BookBuilder {
            context,
            references: BTreeMap::new(),
        }
    }

    pub fn contains_file<P: AsRef<Path>>(&self, p: P) -> bool {
        let f = p.as_ref();
        if let Ok(c) = f.canonicalize() {
            for b in self.references.keys() {
                if c.starts_with(b.as_path()) {
                    debug!("ignore book file {}", f.display());
                    return true;
                }
            }
        }
        false
    }

    pub fn get_book_config<P: AsRef<Path>>(&self, p: P) -> PathBuf {
        let mut book = p.as_ref().to_path_buf();
        book.push(BOOK_TOML);
        book
    }

    pub fn is_book_dir<P: AsRef<Path>>(&self, p: P) -> bool {
        let book = self.get_book_config(p);
        if book.exists() {
            return true;
        }
        false
    }

    fn copy_book(&self, source_dir: &Path, build_dir: PathBuf) -> Result<(), Error> {
        // Jump some hoops to bypass the book build_dir
        let relative = source_dir.strip_prefix(&self.context.options.source)?;
        let mut base = self.context.options.target.clone();
        base.push(relative);

        let build = self.context.config.build.as_ref().unwrap();
        let follow_links = build.follow_links.is_some() && build.follow_links.unwrap();

        for result in WalkBuilder::new(&build_dir)
            .follow_links(follow_links)
            .build()
        {
            match result {
                Ok(entry) => {
                    if entry.path().is_file() {
                        let file = entry.path().to_path_buf();
                        // Get a relative file and append it to the correct output base directory
                        let dest = file.strip_prefix(&build_dir).unwrap();
                        let mut output = base.clone();
                        output.push(dest);

                        // TODO: minify files with HTML file extension

                        // Copy the file content
                        if let Err(e) = utils::fs::copy(file, output) {
                            return Err(Error::IoError(e));
                        }
                    }
                }
                Err(e) => return Err(Error::IgnoreError(e)),
            }
        }

        Ok(())
    }

    fn is_draft<P: AsRef<Path>>(&self, p: P) -> bool {
        let dir = p.as_ref();
        let mut is_draft = false;
        if self.context.options.release {
            let conf_result = loader::load_toml_to_json(self.get_book_config(&dir));
            match conf_result {
                Ok(map) => {
                    if let Some(site) = map.get(BOOK_SITE_TABLE_KEY) {
                        if let Some(draft) = site.get(DRAFT_KEY) {
                            if let Some(val) = draft.as_bool() {
                                is_draft = val;
                            }
                        }
                    }
                }
                Err(_) => (),
            }
        }
        return is_draft;
    }

    pub fn load<P: AsRef<Path>>(&mut self, context: &Context, p: P) -> Result<(), Error> {
        let dir = p.as_ref();
        let directory = dir.canonicalize()?;

        info!("load {}", dir.display());

        let result = MDBook::load(dir);
        match result {
            Ok(mut md) => {
                let theme = self
                    .context
                    .config
                    .get_book_theme_path(&self.context.options.source);

                if let Some(theme_dir) = theme {
                    if theme_dir.exists() && theme_dir.is_dir() {
                        if let Some(s) = theme_dir.to_str() {
                            md.config.set(BOOK_THEME_KEY, s)?;
                        }
                    } else {
                        warn!("Missing book theme directory '{}'", theme_dir.display());
                    }
                }

                if let Some(ref livereload_url) = context.livereload {
                    md.config
                        .set("output.html.livereload-url", livereload_url)?;
                }

                self.references.insert(directory, md);
            }
            Err(e) => return Err(Error::BookError(e)),
        }

        Ok(())
    }

    pub fn build<P: AsRef<Path>>(&self, p: P) -> Result<(), Error> {
        let dir = p.as_ref();
        let directory = dir.canonicalize()?;
        if let Some(md) = self.references.get(&directory) {
            if self.is_draft(&dir) {
                info!("draft book skipped {}", dir.display());
                return Ok(());
            }

            info!("build {}", dir.display());

            let built = md.build();
            match built {
                Ok(_) => {
                    let bd = &md.config.build.build_dir;
                    let mut src = dir.to_path_buf();
                    src.push(bd);
                    self.copy_book(dir, src)
                }
                Err(e) => return Err(Error::BookError(e)),
            }
        } else {
            return Err(Error::new(format!("No book found for {}", dir.display())));
        }
    }

    pub fn rebuild<P: AsRef<Path>>(&mut self, context: &Context, p: P) -> Result<(), Error> {
        // NOTE: unfortunately mdbook requires a reload before a build
        self.load(context, p.as_ref())?;
        self.build(p)
    }

    pub fn all(&mut self, context: &Context) -> Result<(), Error> {
        let paths = self
            .references
            .keys()
            .map(|p| p.clone())
            .collect::<Vec<_>>();

        for p in paths {
            self.rebuild(context, p)?;
        }
        Ok(())
    }
}
