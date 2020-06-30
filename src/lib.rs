#[macro_use]
extern crate lazy_static;

pub mod bundler;

mod build;
mod cache;
mod command;
mod config;
mod error;
mod git;
mod locale;
mod markdown;
mod preference;
pub mod publisher;
pub mod updater;
mod workspace;

static INDEX_STEM: &str = "index";
static INDEX_HTML: &str = "index.html";
static TEMPLATE_EXT: &str = ".hbs";
static LAYOUT_HBS: &str = "layout.hbs";
static MD: &str = "md";
static HTML: &str = "html";
static JSON: &str = "json";
static DRAFT_KEY: &str = "draft";

// FIXME: remove these and their usages
static PARSE_EXTENSIONS: [&str; 2] = [HTML, MD];

pub use crate::command::blueprint;
pub use crate::command::build::build_project;
pub use crate::command::docs;
pub use crate::command::fetch;
pub use crate::command::run;
pub use crate::command::pref;
pub use crate::command::publish;
pub use crate::command::site;
pub use crate::command::upgrade;

pub use crate::config::{BuildArguments, Config};
pub use crate::error::{AwsError, Error};

pub type ErrorCallback = fn(Error);
pub type Result<T> = std::result::Result<T, crate::error::Error>;
pub type AwsResult<T> = std::result::Result<T, crate::error::AwsError>;
