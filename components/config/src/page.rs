use std::io;
use std::mem;
use std::path::PathBuf;

use chrono::prelude::*;

use serde::{Deserialize, Serialize, Deserializer};
use serde_json::{Map, Value};
use serde_with::skip_serializing_none;

/// Used as an attribute when we want to convert from TOML to a string date
pub fn from_toml_datetime<'de, D>(deserializer: D) 
    -> Result<Option<DateTime<Utc>>, D::Error> where D: Deserializer<'de> {

    toml::value::Datetime::deserialize(deserializer).map(|s| {
        Some(
            DateTime::<Utc>::from_utc(
                NaiveDateTime::parse_from_str(&s.to_string(), "%Y-%m-%d")
                .unwrap_or(Utc::now().naive_utc()),
                Utc
            )
        )
    })
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileContext {
    pub source: PathBuf,
    pub target: PathBuf,
    pub name: Option<String>,
    pub modified: DateTime<Utc>,
}

impl FileContext {
    pub fn new(source: PathBuf, target: PathBuf) -> Self {
        let mut name = None;
        if let Some(stem) = &source.file_stem() {
            name = Some(stem.to_string_lossy().into_owned());
        }

        Self {
            source,
            target,
            name,
            modified: Utc::now(),
        }
    }

    pub fn resolve_metadata(&mut self) -> io::Result<()> {
        if let Ok(ref metadata) = self.source.metadata() {
            if let Ok(modified) = metadata.modified() {
                self.modified = DateTime::from(modified);
            }
        }
        Ok(())
    }
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct Page {

    // Configurable
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub author: Option<Author>,
    pub rewrite_index: Option<bool>,
    pub draft: Option<bool>,
    pub standalone: Option<bool>,
    pub query: Option<Value>,
    pub layout: Option<PathBuf>,
    pub tags: Option<Vec<String>>,

    pub scripts: Option<Vec<String>>,
    pub styles: Option<Vec<String>>,

    #[serde(deserialize_with = "from_toml_datetime")]
    pub created: Option<DateTime<Utc>>,

    // NOTE: that we do not define `context` as it would
    // NOTE: create a recursive data type; the template
    // NOTE: logic should inject it into `vars`
    #[serde(flatten)]
    pub extra: Map<String, Value>,

    // Reserved
    #[serde(skip_deserializing)]
    pub href: Option<String>,
    #[serde(skip_deserializing)]
    pub lang: Option<String>,
    #[serde(skip_deserializing)]
    pub file: Option<FileContext>,

    // Layout template data
    #[serde(skip_deserializing)]
    pub template: Option<String>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            keywords: None,
            author: None,
            rewrite_index: None,
            draft: Some(false),
            standalone: Some(false),
            query: None,
            layout: None,
            tags: None,
            scripts: None,
            styles: None,
            created: None,

            extra: Map::new(),

            href: None,
            lang: None,
            file: None,
            template: None,
        }
    }
}

impl Page {
    pub fn append(&mut self, other: &mut Self) {
        if let Some(title) = other.title.as_mut() {
            self.title = Some(mem::take(title));
        }

        if let Some(description) = other.description.as_mut() {
            self.description = Some(mem::take(description));
        }

        if let Some(keywords) = other.keywords.as_mut() {
            self.keywords = Some(mem::take(keywords));
        }

        if let Some(author) = other.author.as_mut() {
            self.author = Some(mem::take(author));
        }

        if let Some(rewrite_index) = other.rewrite_index.as_mut() {
            self.rewrite_index = Some(mem::take(rewrite_index));
        }

        if let Some(draft) = other.draft.as_mut() {
            self.draft = Some(mem::take(draft));
        }

        if let Some(standalone) = other.standalone.as_mut() {
            self.standalone = Some(mem::take(standalone));
        }

        if let Some(query) = other.query.as_mut() {
            self.query = Some(mem::take(query));
        }

        if let Some(layout) = other.layout.as_mut() {
            self.layout = Some(mem::take(layout));
        }

        if let Some(tags) = other.tags.as_mut() {
            self.tags = Some(mem::take(tags));
        }

        if let Some(scripts) = other.scripts.as_mut() {
            self.scripts = Some(mem::take(scripts));
        }

        if let Some(styles) = other.styles.as_mut() {
            self.styles = Some(mem::take(styles));
        }

        self.created = other.created.clone();

        if let Some(href) = other.href.as_mut() {
            self.href = Some(mem::take(href));
        }

        self.extra.append(&mut other.extra);
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Author {
    pub name: String,
    pub url: Option<String>,
}
