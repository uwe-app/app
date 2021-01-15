use std::collections::HashMap;
use std::path::PathBuf;

use url::Url;

use serde::Serialize;
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use config::{
    date::DateConfig, page::Page, repository::RepositoryConfig,
    semver::Version, Author, Config, RuntimeOptions,
};

use locale::{LocaleMap, Locales};

use crate::{Error, Result};

#[serde_as]
#[skip_serializing_none]
#[derive(Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CollatedPage<'config, 'locale> {
    #[serde(flatten)]
    page: &'config Page,

    #[serde_as(as = "DisplayFromStr")]
    permalink: Url,

    lang: &'config str,
    charset: &'config str,
    host: &'config str,
    #[serde_as(as = "DisplayFromStr")]
    website: &'config Url,

    languages: Option<&'locale LocaleMap>,

    date: &'config Option<DateConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    repository: &'config Option<RepositoryConfig>,

    #[serde(skip_serializing_if = "CollatedAuthors::is_empty")]
    authors: CollatedAuthors<'config>,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub menus: HashMap<String, Vec<String>>,
    //pub menus: HashMap<&'collation String, Vec<&'collation String>>,
    generator: &'config str,
    #[serde_as(as = "DisplayFromStr")]
    version: &'config Version,

    #[serde(skip_serializing_if = "Option::is_none")]
    commit: &'config Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    members: &'config Option<HashMap<String, String>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(default)]
pub struct CollatedAuthors<'config> {
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    all: &'config HashMap<String, Author>,
    attributed: Option<Vec<&'config Author>>,
}

impl CollatedAuthors<'_> {
    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }
}

impl<'config, 'locale> CollatedPage<'config, 'locale> {
    pub fn new(
        file: &PathBuf,
        config: &'config Config,
        options: &RuntimeOptions,
        locales: &'locale Locales,
        page: &'config Page,
        lang: &'config str,
    ) -> Result<Self> {
        let languages = if locales.is_multi_lingual() {
            Some(locales.languages())
        } else {
            None
        };

        let attributed = if let Some(author_refs) = page.authors() {
            let authors = config
                .authors()
                .iter()
                .filter(|(k, _)| author_refs.contains(k))
                .map(|(_, v)| v)
                .collect::<Vec<_>>();

            if authors.len() != author_refs.len() {
                let missing = author_refs
                    .iter()
                    .filter(|r| !config.authors().contains_key(r.as_str()))
                    .cloned()
                    .collect::<Vec<String>>();
                return Err(Error::NoAuthor(missing.join(", "), file.clone()));
            }

            Some(authors)
        } else {
            None
        };

        let authors = CollatedAuthors {
            all: config.authors(),
            attributed,
        };

        let commit = if options.settings.include_commit() {
            config.commit()
        } else {
            &None
        };

        Ok(Self {
            page,
            lang,
            charset: config.charset(),
            permalink: page.permalink(config, options)?,
            host: config.host(),
            website: config.website(),
            repository: config.repository(),
            authors,
            languages,
            date: &config.date,
            menus: Default::default(),
            generator: config::generator::user_agent(),
            version: config.version(),
            commit,
            members: config.member_urls(),
        })
    }

    pub fn page(&self) -> &'config Page {
        self.page
    }
}
