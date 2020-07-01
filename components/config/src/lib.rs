use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
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

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
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
