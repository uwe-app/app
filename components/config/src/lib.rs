#[macro_use]
extern crate log;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Lang(#[from] unic_langid::LanguageIdentifierError),
}

impl ConfigError {
    pub fn new(s: String) -> Self {
        ConfigError::Message(s)
    }
}

pub mod config;
pub mod page;

pub use config::*;

//#[cfg(test)]
//mod tests {
    //#[test]
    //fn it_works() {
        //assert_eq!(2 + 2, 4);
    //}
//}
