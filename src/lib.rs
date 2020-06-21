#[macro_use]
extern crate lazy_static;

mod build;
mod bundle;
mod cache;
pub mod callback;
mod command;
mod config;
mod content;
mod error;
mod git;
mod locale;
mod preference;
mod server;
mod utils;
mod workspace;

static INDEX_STEM: &str = "index";
static INDEX_HTML: &str = "index.html";
static TEMPLATE_EXT: &str = ".hbs";
static LAYOUT_HBS: &str = "layout.hbs";
static MD: &str = "md";
static HTML: &str = "html";
static JSON: &str = "json";
static DRAFT_KEY: &str = "draft";
static BOOK_TOML: &str = "book.toml";
static BOOK_THEME_KEY: &str = "output.html.theme";

// FIXME: remove these and their usages
static PARSE_EXTENSIONS: [&str; 2] = [HTML, MD];
static DATA_TOML: &str = "data.toml";

pub use crate::config::{Config, BuildArguments};
pub use crate::error::Error;
pub use crate::command::build::*;
pub use crate::command::bundle::*;
pub use crate::command::init::*;
pub use crate::command::pref::*;
pub use crate::command::serve::*;
pub use crate::command::update::*;
pub use crate::utils::generate_id;
