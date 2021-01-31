use std::collections::HashMap;
use std::path::Path;

use thiserror::Error;

use serde::Serialize;

use fluent_templates::{static_loader, ArcLoader, Loader};
use unic_langid::LanguageIdentifier;

use once_cell::sync::OnceCell;

use config::{Config, FluentConfig};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    LanguageIdentifier(#[from] unic_langid::LanguageIdentifierError),
    #[error("{0}")]
    Message(String),
}

type Result<T> = std::result::Result<T, Error>;

static_loader! {
    pub static LOCALES = {locales: "./locales", fallback_language: "en"};
}

pub type LocaleName = String;
pub type LocaleIdentifier = HashMap<LocaleName, LanguageIdentifier>;

#[derive(Debug, Clone, Default, Serialize)]
pub struct LocaleMap {
    /// The fallback language is inherited
    /// from the top-level config `lang`
    fallback: LocaleName,
    /// List of languages other than the primary fallback language.
    alternate: Vec<String>,

    // NOTE: We don't include `multi` when serializing because
    // NOTE: serialization of `languages` is already conditional
    // NOTE: on whether multiple languages are available.
    /// Determine if multiple locales are active.
    #[serde(skip)]
    multi: bool,

    /// Enabled is active when we are able to load
    /// locale files at runtime.
    #[serde(skip)]
    enabled: bool,

    /// Map of all locales to the parsed language identifiers.
    #[serde(skip)]
    map: LocaleIdentifier,
}

impl LocaleMap {
    /// Determine if this project has multiple languages.
    pub fn is_multi_lingual(&self) -> bool {
        self.multi
    }

    /// Get all locale identifiers excluding the fallback.
    pub fn alternate(&self) -> &Vec<String> {
        &self.alternate
    }
}

#[derive(Debug, Default)]
pub struct Locales {
    languages: LocaleMap,
}

impl Locales {
    fn get_locale_map(
        &self,
        arc: &Option<Box<ArcLoader>>,
        fallback: &str,
    ) -> Result<LocaleMap> {
        let mut res = LocaleMap {
            fallback: fallback.to_string(),
            map: HashMap::new(),
            enabled: arc.is_some(),
            multi: false,
            alternate: vec![],
        };

        if let Some(ref arc) = arc {
            let langs = arc.locales();
            for lang_id in langs {
                res.map.insert(lang_id.to_string(), lang_id.clone());
            }
        } else {
            let id: LanguageIdentifier = fallback.parse()?;
            res.map.insert(fallback.to_string(), id);
        }

        res.multi = res.map.len() > 1;

        let alternate: Vec<String> = res
            .map
            .keys()
            .filter(|s| s.as_str() != fallback)
            .map(|s| s.to_owned())
            .collect();

        res.alternate = alternate;

        Ok(res)
    }

    fn init<P>(
        &mut self,
        config: &Config,
        path: P,
    ) -> (Option<Box<ArcLoader>>, Option<Box<dyn std::error::Error>>)
    where
        P: AsRef<Path>,
    {
        let locales_dir = path.as_ref();
        if locales_dir.exists() && locales_dir.is_dir() {
            match arc(locales_dir, config.fluent()) {
                Ok(result) => {
                    return (Some(Box::new(result)), None);
                }
                Err(e) => {
                    return (None, Some(e));
                }
            }
        }
        (None, None)
    }

    fn wrap(
        &self,
        loader: Option<Box<ArcLoader>>,
    ) -> &'static Option<Box<ArcLoader>> {
        static CELL: OnceCell<Option<Box<ArcLoader>>> = OnceCell::new();
        CELL.get_or_init(|| loader)
    }

    pub fn languages(&self) -> &LocaleMap {
        &self.languages
    }

    pub fn is_multi_lingual(&self) -> bool {
        self.languages().multi
    }

    pub fn loader(&self) -> &'static Option<Box<ArcLoader>> {
        self.wrap(None)
    }

    pub fn load<P>(&mut self, config: &Config, path: P) -> Result<&LocaleMap>
    where
        P: AsRef<Path>,
    {
        let arc = match self.init(config, path) {
            (arc, err) => {
                match err {
                    Some(e) => return Err(Error::Message(e.to_string())),
                    _ => {}
                }
                arc
            }
        };

        let arc_ref = self.wrap(arc);

        self.languages = self.get_locale_map(arc_ref, config.lang())?;
        Ok(&self.languages)
    }
}

fn arc<'a, P: AsRef<Path>>(
    dir: P,
    fluent: &FluentConfig,
) -> std::result::Result<ArcLoader, Box<dyn std::error::Error>> {
    let file = dir.as_ref();
    let mut core = file.to_path_buf();
    core.push(fluent.shared());
    ArcLoader::builder(dir.as_ref(), fluent.fallback().clone())
        .shared_resources(Some(&[core]))
        .build()
}
