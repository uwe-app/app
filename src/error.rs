use std::{error, fmt, io, path};

use handlebars;
use ignore;
use mdbook;

#[derive(Debug)]
pub enum Error {
    Message(String),
    IoError(io::Error),
    StripPrefixError(path::StripPrefixError),
    TemplateFileError(handlebars::TemplateFileError),
    RenderError(handlebars::RenderError),
    IgnoreError(ignore::Error),
    BookError(mdbook::errors::Error),
    TomlSerError(toml::ser::Error),
    TomlDeserError(toml::de::Error),
    JsonError(serde_json::error::Error),
    NotifyError(notify::Error),
    UrlParseError(url::ParseError),
    LanguageIdentifierError(unic_langid::LanguageIdentifierError),
    // For fluent template loader
    Boxed(Box<dyn std::error::Error>),
    Git(git2::Error),
    Semver(semver::SemVerError),
    HttpClient(reqwest::Error),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<path::StripPrefixError> for Error {
    fn from(error: path::StripPrefixError) -> Self {
        Error::StripPrefixError(error)
    }
}

impl From<handlebars::TemplateFileError> for Error {
    fn from(error: handlebars::TemplateFileError) -> Self {
        Error::TemplateFileError(error)
    }
}

impl From<handlebars::RenderError> for Error {
    fn from(error: handlebars::RenderError) -> Self {
        Error::RenderError(error)
    }
}

impl From<toml::de::Error> for Error {
    fn from(error: toml::de::Error) -> Self {
        Error::TomlDeserError(error)
    }
}

impl From<toml::ser::Error> for Error {
    fn from(error: toml::ser::Error) -> Self {
        Error::TomlSerError(error)
    }
}

impl From<ignore::Error> for Error {
    fn from(error: ignore::Error) -> Self {
        Error::IgnoreError(error)
    }
}

impl From<mdbook::errors::Error> for Error {
    fn from(error: mdbook::errors::Error) -> Self {
        Error::BookError(error)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(error: serde_json::error::Error) -> Self {
        Error::JsonError(error)
    }
}

impl From<notify::Error> for Error {
    fn from(error: notify::Error) -> Self {
        Error::NotifyError(error)
    }
}

impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Self {
        Error::UrlParseError(error)
    }
}

impl From<unic_langid::LanguageIdentifierError> for Error {
    fn from(error: unic_langid::LanguageIdentifierError) -> Self {
        Error::LanguageIdentifierError(error)
    }
}

impl From<Box<dyn std::error::Error>> for Error {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        Error::Boxed(error)
    }
}

impl From<git2::Error> for Error {
    fn from(error: git2::Error) -> Self {
        Error::Git(error)
    }
}

impl From<semver::SemVerError> for Error {
    fn from(error: semver::SemVerError) -> Self {
        Error::Semver(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::HttpClient(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Message(ref s) => write!(f, "{}", s),
            Error::IoError(ref e) => e.fmt(f),
            Error::StripPrefixError(ref e) => e.fmt(f),
            Error::TemplateFileError(ref e) => e.fmt(f),
            Error::RenderError(ref e) => e.fmt(f),
            Error::IgnoreError(ref e) => e.fmt(f),
            Error::BookError(ref e) => e.fmt(f),
            Error::TomlSerError(ref e) => e.fmt(f),
            Error::TomlDeserError(ref e) => e.fmt(f),
            Error::JsonError(ref e) => e.fmt(f),
            Error::NotifyError(ref e) => e.fmt(f),
            Error::UrlParseError(ref e) => e.fmt(f),
            Error::LanguageIdentifierError(ref e) => e.fmt(f),
            Error::Boxed(ref e) => e.fmt(f),
            Error::Git(ref e) => e.fmt(f),
            Error::Semver(ref e) => e.fmt(f),
            Error::HttpClient(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::IoError(ref e) => Some(e),
            Error::StripPrefixError(ref e) => Some(e),
            Error::TemplateFileError(ref e) => Some(e),
            Error::RenderError(ref e) => Some(e),
            Error::IgnoreError(ref e) => Some(e),
            Error::BookError(ref e) => Some(e),
            Error::TomlSerError(ref e) => Some(e),
            Error::TomlDeserError(ref e) => Some(e),
            Error::JsonError(ref e) => Some(e),
            Error::NotifyError(ref e) => Some(e),
            Error::UrlParseError(ref e) => Some(e),
            Error::LanguageIdentifierError(ref e) => Some(e),
            Error::Git(ref e) => Some(e),
            Error::Semver(ref e) => Some(e),
            Error::HttpClient(ref e) => Some(e),
            _ => None,
        }
    }
}
