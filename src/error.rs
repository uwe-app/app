use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    InvalidUri(#[from] warp::http::uri::InvalidUri),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    TomlSer(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeser(#[from] toml::de::Error),
    #[error(transparent)]
    Json(#[from] serde_json::error::Error),

    #[error(transparent)]
    UrlParse(#[from] url::ParseError),

    #[error(transparent)]
    LanguageIdentifier(#[from] unic_langid::LanguageIdentifierError),

    #[error(transparent)]
    Git(#[from] git2::Error),
    #[error(transparent)]
    Semver(#[from] semver::SemVerError),

    #[error(transparent)]
    Compiler(#[from] compiler::Error),
    #[error(transparent)]
    Locale(#[from] locale::Error),

    #[error(transparent)]
    GitLib(#[from] git::error::GitError),
    #[error(transparent)]
    Preference(#[from] preference::PreferenceError),
    #[error(transparent)]
    Cache(#[from] cache::CacheError),
    #[error(transparent)]
    Updater(#[from] updater::UpdaterError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error(transparent)]
    Report(#[from] report::ReportError),
    #[error(transparent)]
    Aws(#[from] publisher::AwsError),
}

impl Error {
    pub fn new(s: String) -> Self {
        Error::Message(s)
    }
}
