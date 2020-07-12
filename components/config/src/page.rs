use std::io;
use std::mem;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use chrono::prelude::*;

use serde::{Deserialize, Serialize, Deserializer};
use serde_json::{json, Map, Value};
use serde_with::skip_serializing_none;

use super::Error;
use super::link;
use super::{Config, FileInfo, RuntimeOptions};
use super::indexer::QueryList;

/// Attribute to convert from TOML date time to chronos UTC variant
pub fn from_toml_datetime<'de, D>(deserializer: D) 
    -> Result<Option<DateTime<Utc>>, D::Error> where D: Deserializer<'de> {

    toml::value::Datetime::deserialize(deserializer).map(|s| {
        let d = s.to_string();
        let dt = if d.contains('T') {
            DateTime::parse_from_rfc3339(&d).ok().map(|s| s.naive_local())
        } else {
            NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok().map(|s| s.and_hms(0, 0, 0))
        };

        if let Some(dt) = dt {
            return Some(DateTime::<Utc>::from_utc(dt, Utc))
        }

        None
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

    //
    // Configurable
    // 
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,

    pub render: Option<bool>,
    pub rewrite_index: Option<bool>,
    pub draft: Option<bool>,
    pub standalone: Option<bool>,
    pub listing: Option<bool>,

    pub authors: Option<Vec<Author>>,
    pub byline: Option<Vec<String>>,

    pub query: Option<QueryList>,

    pub layout: Option<PathBuf>,
    pub tags: Option<Vec<String>>,
    pub meta: Option<HashMap<String, Vec<String>>>,

    pub scripts: Option<Vec<String>>,
    pub styles: Option<Vec<String>>,

    //#[serde(deserialize_with = "from_toml_datetime", serialize_with = "to_toml_datetime")]
    #[serde(deserialize_with = "from_toml_datetime")]
    pub created: Option<DateTime<Utc>>,

    //#[serde(deserialize_with = "from_toml_datetime", serialize_with = "to_toml_datetime")]
    #[serde(deserialize_with = "from_toml_datetime")]
    pub updated: Option<DateTime<Utc>>,

    //
    // Reserved
    // 
    #[serde(skip_deserializing)]
    pub href: Option<String>,
    #[serde(skip_deserializing)]
    pub lang: Option<String>,
    #[serde(skip_deserializing)]
    pub file: Option<FileContext>,

    // Layout template data
    #[serde(skip_deserializing)]
    pub template: Option<String>,

    // NOTE: that we do not define `context` as it would
    // NOTE: create a recursive data type; the template
    // NOTE: logic should inject it into `vars`
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            keywords: None,
            authors: None,
            byline: None,
            rewrite_index: None,
            render: Some(true),
            draft: Some(false),
            standalone: Some(false),
            listing: Some(true),
            query: None,
            layout: None,
            tags: None,
            meta: None,
            scripts: None,
            styles: None,

            created: None,
            updated: None,

            extra: Map::new(),

            href: None,
            lang: None,
            file: None,
            template: None,
        }
    }
}

impl Page {

    pub fn finalize<L: AsRef<str>>(&mut self, lang: L, info: &FileInfo, config: &Config) -> Result<(), Error> {
        let output = info.output.as_ref().unwrap();

        self.lang = Some(lang.as_ref().to_string());

        let mut file_context = FileContext::new(info.file.clone(), output.clone());
        file_context.resolve_metadata()?;

        // TODO: allow setting to control this behavior
        if self.updated.is_none() {
            self.updated = Some(file_context.modified.clone());
        }

        self.file = Some(file_context);

        // Some useful shortcuts
        if let Some(ref date) = config.date {
            self.extra.insert("date-formats".to_string(), json!(date.formats));
        }

        Ok(())
    }

    pub fn compute<P: AsRef<Path>>(&mut self, p: P, config: &Config, opts: &RuntimeOptions) -> Result<(), Error> {

        self.href = Some(link::absolute(p.as_ref(), config, opts, Default::default())?);

        let mut authors_list = if let Some(ref author) = self.authors {
            author.clone()
        } else {
            Vec::new()
        };

        // TODO: finalize this page data after computation 
        // TODO: build dynamic sort keys like date tuple (year, month, day) etc.

        if let Some(ref list) = self.byline {
            for id in list {
                if let Some(ref authors) = config.authors {
                    if let Some(author) = authors.get(id) {
                        authors_list.push(author.clone());
                    } else {
                        return Err(Error::NoAuthor(id.to_string()))
                    }
                } else {
                    return Err(Error::NoAuthor(id.to_string()))
                }
            }
        }

        self.authors = Some(authors_list);

        Ok(())
    }

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

        if let Some(render) = other.render.as_mut() {
            self.render = Some(mem::take(render));
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

        if let Some(listing) = other.listing.as_mut() {
            self.listing = Some(mem::take(listing));
        }

        if let Some(authors) = other.authors.as_mut() {
            self.authors = Some(mem::take(authors));
        }

        if let Some(byline) = other.byline.as_mut() {
            self.byline = Some(mem::take(byline));
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

        if let Some(meta) = other.meta.as_mut() {
            self.meta = Some(mem::take(meta));
        }

        if let Some(scripts) = other.scripts.as_mut() {
            self.scripts = Some(mem::take(scripts));
        }

        if let Some(styles) = other.styles.as_mut() {
            self.styles = Some(mem::take(styles));
        }

        self.created = other.created.clone();
        self.updated = other.updated.clone();

        if let Some(href) = other.href.as_mut() {
            self.href = Some(mem::take(href));
        }

        self.extra.append(&mut other.extra);
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Author {
    pub name: String,
    pub link: Option<String>,
}

