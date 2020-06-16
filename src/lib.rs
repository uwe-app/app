#[macro_use]
extern crate lazy_static;

mod asset;
mod blueprint;
mod build;
mod content;
pub mod callback;
mod config;
mod bundle;
mod error;
mod command;
mod locale;
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

pub use crate::config::Config;
pub use crate::error::Error;
pub use crate::command::archive::*;
pub use crate::command::build::*;
pub use crate::command::bundle::*;
pub use crate::command::init::*;
pub use crate::command::serve::*;
pub use crate::utils::generate_id;
