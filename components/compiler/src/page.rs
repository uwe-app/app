use std::collections::HashMap;
use std::path::PathBuf;

use url::Url;

use config::{semver::Version, Author};
use serde::Serialize;
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use config::{date::DateConfig, page::Page, Config, repository::RepositoryConfig};

use locale::{LocaleMap, Locales};

use crate::{Error, Result};

#[serde_as]
#[skip_serializing_none]
#[derive(Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CollatedPage<'config, 'locale> {
    #[serde(flatten)]
    page: &'config Page,

    lang: &'config str,
    charset: &'config str,
    host: &'config str,
    #[serde_as(as = "DisplayFromStr")]
    website: &'config Url,

    languages: Option<&'locale LocaleMap>,

    date: &'config Option<DateConfig>,
    repository: &'config Option<RepositoryConfig>,

    authors: CollatedAuthors<'config>,

    // Paths referenced in a menu when MENU.md convention is used
    //  FIXME: use a better name for the main menu
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub main: Vec<&'config String>,
    pub menus: HashMap<&'config String, Vec<&'config String>>,

    generator: &'config str,
    #[serde_as(as = "DisplayFromStr")]
    version: &'config Version,
}

#[derive(Debug, Serialize)]
#[serde(default)]
pub struct CollatedAuthors<'config> {
    all: &'config HashMap<String, Author>,
    attributed: Option<Vec<&'config Author>>,
}

impl<'config, 'locale> CollatedPage<'config, 'locale> {
    pub fn new(
        file: &PathBuf,
        config: &'config Config,
        locales: &'locale Locales,
        page: &'config Page,
        lang: &'config str,
    ) -> Result<Self> {

        let languages = if locales.is_multi_lingual() {
            Some(locales.languages())
        } else { None };

        let attributed = if let Some(author_refs) = page.authors() {
            let authors = config
                .authors()
                .iter()
                .filter(|(k, _)| author_refs.contains(k))
                .map(|(_, v)| v)
                .collect::<Vec<_>>();

            if authors.len() != author_refs.len() {
                let missing =
                    author_refs
                    .iter()
                    .filter(|r| !config.authors().contains_key(r.as_str()))
                    .cloned()
                    .collect::<Vec<String>>();
                return Err(Error::NoAuthor(missing.join(", "), file.clone()))
            }

            Some(authors)
        } else { None };

        let authors = CollatedAuthors {
            all: config.authors(),
            attributed,
        };

        Ok(Self {
            page,
            lang,
            charset: config.charset(),
            host: config.host(),
            website: config.website(),
            repository: config.repository(),
            authors,
            languages,
            date: &config.date,
            main: Default::default(),
            menus: Default::default(),
            generator: config::generator::id(),
            version: config.version(),
        })
    }
}
