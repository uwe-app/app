use std::{error, fmt, io, path};

use warp::http;

use cache;
use git::error::GitError;

use publisher::AwsError;

#[derive(Debug)]
pub enum Error {
    Message(String),
    Uri(http::uri::InvalidUri),
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
    GitLib(GitError),
    Git(git2::Error),
    Semver(semver::SemVerError),
    Preference(preference::PreferenceError),
    Cache(cache::CacheError),
    Updater(updater::UpdaterError),
    Config(config::ConfigError),
    Report(report::ReportError),
    Aws(AwsError),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}


impl From<http::uri::InvalidUri> for Error {
    fn from(error: http::uri::InvalidUri) -> Self {
        Error::Uri(error)
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

impl From<GitError> for Error {
    fn from(error: GitError) -> Self {
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

impl From<AwsError> for Error {
    fn from(error: AwsError) -> Self {
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

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::IoError(ref e) => Some(e),
            Error::Uri(ref e) => Some(e),
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
            Error::GitLib(ref e) => Some(e),
            Error::Semver(ref e) => Some(e),
            Error::Preference(ref e) => Some(e),
            Error::Cache(ref e) => Some(e),
            Error::Updater(ref e) => Some(e),
            Error::Config(ref e) => Some(e),
            Error::Report(ref e) => Some(e),
            Error::Aws(ref e) => Some(e),
            _ => None,
        }
    }
}

//#[derive(Debug)]
//pub enum AwsError {
    //Io(io::Error),
    //Tls(rusoto_core::request::TlsError),
    //Credentials(rusoto_core::credential::CredentialsError),
    //HeadBucket(rusoto_core::RusotoError<rusoto_s3::HeadBucketError>),
    //PutObject(rusoto_core::RusotoError<rusoto_s3::PutObjectError>),
    //DeleteObject(rusoto_core::RusotoError<rusoto_s3::DeleteObjectError>),
    //ListObjects(rusoto_core::RusotoError<rusoto_s3::ListObjectsV2Error>),
//}

//impl From<io::Error> for AwsError {
    //fn from(error: io::Error) -> Self {
        //AwsError::Io(error)
    //}
//}

//impl From<rusoto_core::request::TlsError> for AwsError {
    //fn from(error: rusoto_core::request::TlsError) -> Self {
        //AwsError::Tls(error)
    //}
//}

//impl From<rusoto_core::credential::CredentialsError> for AwsError {
    //fn from(error: rusoto_core::credential::CredentialsError) -> Self {
        //AwsError::Credentials(error)
    //}
//}

//impl From<rusoto_core::RusotoError<rusoto_s3::HeadBucketError>> for AwsError {
    //fn from(error: rusoto_core::RusotoError<rusoto_s3::HeadBucketError>) -> Self {
        //AwsError::HeadBucket(error)
    //}
//}

//impl From<rusoto_core::RusotoError<rusoto_s3::PutObjectError>> for AwsError {
    //fn from(error: rusoto_core::RusotoError<rusoto_s3::PutObjectError>) -> Self {
        //AwsError::PutObject(error)
    //}
//}

//impl From<rusoto_core::RusotoError<rusoto_s3::DeleteObjectError>> for AwsError {
    //fn from(error: rusoto_core::RusotoError<rusoto_s3::DeleteObjectError>) -> Self {
        //AwsError::DeleteObject(error)
    //}
//}

//impl From<rusoto_core::RusotoError<rusoto_s3::ListObjectsV2Error>> for AwsError {
    //fn from(error: rusoto_core::RusotoError<rusoto_s3::ListObjectsV2Error>) -> Self {
        //AwsError::ListObjects(error)
    //}
//}

//impl fmt::Display for AwsError {
    //fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //match *self {
            //AwsError::Io(ref e) => e.fmt(f),
            //AwsError::Tls(ref e) => e.fmt(f),
            //AwsError::Credentials(ref e) => e.fmt(f),
            //AwsError::HeadBucket(ref e) => e.fmt(f),
            //AwsError::PutObject(ref e) => e.fmt(f),
            //AwsError::DeleteObject(ref e) => e.fmt(f),
            //AwsError::ListObjects(ref e) => e.fmt(f),
        //}
    //}
//}

//impl error::Error for AwsError {
    //fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        //match *self {
            //AwsError::Io(ref e) => Some(e),
            //AwsError::Tls(ref e) => Some(e),
            //AwsError::Credentials(ref e) => Some(e),
            //AwsError::HeadBucket(ref e) => Some(e),
            //AwsError::PutObject(ref e) => Some(e),
            //AwsError::DeleteObject(ref e) => Some(e),
            //AwsError::ListObjects(ref e) => Some(e),
        //}
    //}
//}

