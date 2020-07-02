use std::path::Path;

use thiserror::Error;
use config::Config;
use log::info;

use mdbook::MDBook;

pub mod compiler;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    Book(#[from] mdbook::errors::Error),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}

type Result<T> = std::result::Result<T, Error>;

// List books in the project
pub fn list(config: &Config) -> Result<()> {
    info!("List books in {}", config.get_project().display());
    if let Some(ref book) = config.book {
        if !book.members.is_empty() {
            for (group, members) in &book.members {
                for (name, cfg) in members {
                    info!("{}.{} -> {}", group, name, cfg.path.display());
                }
            }
            return Ok(())
        }
    }
    info!("No books yet");
    Ok(())
}

// Create a new book
pub fn add<P: AsRef<Path>>(
    config: &Config,
    dir: P,
    title: Option<String>,
    authors: Option<Vec<String>>) -> Result<MDBook> {

    let build_config = config.build.as_ref().unwrap();
    let mut book_dir = build_config.source.clone();
    book_dir.push(dir);

    if book_dir.exists() {
        return Err(
            Error::new(
                format!("Book path exists {}", book_dir.display())))
    }

    // create a default config and change a couple things
    let mut cfg = mdbook::Config::default();
    cfg.book.title = title;
    if let Some(authors) = authors {
        for a in authors {
            cfg.book.authors.push(a);
        }
    }


    Ok(MDBook::init(book_dir)
        .create_gitignore(true)
        .with_config(cfg)
        .build()?)
}

// Build a book, if path is none then build all books
// defined in the config.
pub fn build<P: AsRef<Path>>(config: &Config, path: Vec<P>, release: bool) -> Result<()> {
    let build_config = config.build.as_ref().unwrap();
    let compiler = compiler::BookCompiler::new(
        build_config.source.clone(),
        build_config.target.clone(),
        release
    );

    // Build all the books in the config
    if path.is_empty() {
        compiler.all(config, None)?;
    } else {
        //let root = config.get_project().canonicalize()?;
        //println!("Build specific book! {:?}", root);
        for p in path {
            compiler.build(config, p, None)?;
        }
    }
    Ok(())
}
