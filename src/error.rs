use std::fmt;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    Message(String),
    Uri(warp::http::uri::InvalidUri),
    IoError(std::io::Error),
    StripPrefixError(std::path::StripPrefixError),
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
    GitLib(git::error::GitError),
    Git(git2::Error),
    Semver(semver::SemVerError),
    Preference(preference::PreferenceError),
    Cache(cache::CacheError),
    Updater(updater::UpdaterError),
    Config(config::ConfigError),
    Report(report::ReportError),
    Aws(publisher::AwsError),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}


impl From<warp::http::uri::InvalidUri> for Error {
    fn from(error: warp::http::uri::InvalidUri) -> Self {
        Error::Uri(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<std::path::StripPrefixError> for Error {
    fn from(error: std::path::StripPrefixError) -> Self {
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

impl From<git::error::GitError> for Error {
    fn from(error: git::error::GitError) -> Self {
        Error::GitLib(error)
    }
}

impl From<semver::SemVerError> for Error {
    fn from(error: semver::SemVerError) -> Self {
        Error::Semver(error)
    }
}

impl From<preference::PreferenceError> for Error {
    fn from(error: preference::PreferenceError) -> Self {
        Error::Preference(error)
    }
}

impl From<cache::CacheError> for Error {
    fn from(error: cache::CacheError) -> Self {
        Error::Cache(error)
    }
}

impl From<updater::UpdaterError> for Error {
    fn from(error: updater::UpdaterError) -> Self {
        Error::Updater(error)
    }
}

impl From<config::ConfigError> for Error {
    fn from(error: config::ConfigError) -> Self {
        Error::Config(error)
    }
}

impl From<report::ReportError> for Error {
    fn from(error: report::ReportError) -> Self {
        Error::Report(error)
    }
}

impl From<publisher::AwsError> for Error {
    fn from(error: publisher::AwsError) -> Self {
        Error::Aws(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Message(ref s) => write!(f, "{}", s),
            Error::Uri(ref e) => e.fmt(f),
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
            Error::GitLib(ref e) => e.fmt(f),
            Error::Semver(ref e) => e.fmt(f),
            Error::Preference(ref e) => e.fmt(f),
            Error::Cache(ref e) => e.fmt(f),
            Error::Updater(ref e) => e.fmt(f),
            Error::Config(ref e) => e.fmt(f),
            Error::Report(ref e) => e.fmt(f),
            Error::Aws(ref e) => e.fmt(f),
        }
    }
}

