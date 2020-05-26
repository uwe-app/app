#[macro_use]
extern crate lazy_static;
extern crate open;

mod asset;
mod build;
mod bundle;
mod error;
mod command;
mod tree;
mod utils;

static INDEX_STEM: &str = "index";
static INDEX_HTML: &str = "index.html";
static TEMPLATE: &str = "template";
static TEMPLATE_EXT: &str = ".hbs";
static THEME: &str = "theme";
static LAYOUT_HBS: &str = "layout.hbs";
static DATA_TOML: &str = "data.toml";
static MD: &str = "md";
static HTML: &str = "html";
static PARSE_EXTENSIONS: [&str; 2] = [HTML, MD];
static ROOT_TABLE_KEY: &str = "site";
static DRAFT_KEY: &str = "draft";
static BOOK_TOML: &str = "book.toml";
static BOOK_THEME_KEY: &str = "output.html.theme";

pub use crate::error::Error;
pub use crate::command::init::*;
pub use crate::command::build::*;
pub use crate::command::bundle::*;
pub use crate::command::serve::*;
pub use crate::utils::generate_id;
