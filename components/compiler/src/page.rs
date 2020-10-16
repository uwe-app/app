use std::collections::HashMap;

use url::Url;

use config::semver::Version;
use serde::Serialize;
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use config::{date::DateConfig, page::Page, Config};

use locale::{LocaleMap, Locales};

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

    // Paths referenced in a menu when MENU.md convention is used
    //  FIXME: use a better name for the main menu
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub main: Vec<&'config String>,
    pub menus: HashMap<&'config String, Vec<&'config String>>,

    generator: &'config str,
    #[serde_as(as = "DisplayFromStr")]
    version: &'config Version,
}

impl<'config, 'locale> CollatedPage<'config, 'locale> {
    pub fn new(
        config: &'config Config,
        locales: &'locale Locales,
        page: &'config Page,
        lang: &'config str,
    ) -> Self {

        let languages = if locales.is_multi_lingual() {
            Some(locales.languages())
        } else { None };

        Self {
            page,
            lang,
            charset: config.charset(),
            host: config.host(),
            website: config.website(),
            languages,
            date: &config.date,
            main: Default::default(),
            menus: Default::default(),
            generator: config::generator::id(),
            version: config.version(),
        }
    }
}
