use std::collections::HashMap;

use url::Url;

use config::semver::Version;
use serde::Serialize;
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use config::{
    Config,
    date::DateConfig,
    page::Page,
};

use locale::Locales;

#[serde_as]
#[skip_serializing_none]
#[derive(Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CollatedPage<'a> {
    #[serde(flatten)]
    page: &'a Page,

    lang: &'a str,
    charset: &'a str,
    host: &'a str,
    #[serde_as(as = "DisplayFromStr")]
    website: &'a Url,

    multilingual: bool,

    date: &'a Option<DateConfig>,

    // Paths referenced in a menu when MENU.md convention is used
    //  FIXME: use a better name for the main menu
    pub main: Vec<&'a String>,
    pub menus: HashMap<&'a String, Vec<&'a String>>,

    generator: &'a str,
    #[serde_as(as = "DisplayFromStr")]
    version: &'a Version,
}

impl<'a> CollatedPage<'a> {
    pub fn new(config: &'a Config, locales: &'a Locales, page: &'a Page, lang: &'a str) -> Self {
        Self {
            page,
            lang,
            charset: config.charset(),
            host: config.host(),
            website: config.website(),
            multilingual: locales.is_multi_lingual(),
            date: &config.date,
            main: Default::default(),
            menus: Default::default(),
            generator: config::generator::id(),
            version: config.version(),
        }
    }
}

