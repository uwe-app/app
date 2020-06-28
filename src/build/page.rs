use serde::{Deserialize, Serialize};
use std::mem;
use std::path::PathBuf;

use chrono::DateTime;
use chrono::Utc;
use serde_json::{Map, Value};
use serde_with::skip_serializing_none;

use crate::Error;

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

    pub fn resolve_metadata(&mut self) -> Result<(), Error> {
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
pub struct Page {
    // Configurable
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub author: Option<Author>,
    pub clean: Option<bool>,
    pub draft: Option<bool>,
    pub standalone: Option<bool>,
    pub query: Option<Value>,
    pub layout: Option<PathBuf>,

    // Reserved
    pub lang: Option<String>,
    pub file: Option<FileContext>,

    // Layout template data
    pub template: Option<String>,

    // NOTE: that we do not define `context` as it would
    // NOTE: create a recursive data type; the template
    // NOTE: logic should inject it into `vars`
    #[serde(flatten)]
    pub vars: Map<String, Value>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            keywords: None,
            author: None,
            clean: None,
            draft: Some(false),
            standalone: Some(false),
            query: None,
            layout: None,
            vars: Map::new(),

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

        if let Some(clean) = other.clean.as_mut() {
            self.clean = Some(mem::take(clean));
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

        self.vars.append(&mut other.vars);

        //if let Some(vars) = other.vars.as_mut() {
        //if let Some(self_vars) = self.vars.as_mut() {
        //self_vars.append(vars);
        //} else {
        //self.vars = Some(vars);
        //}

        //}
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Author {
    pub name: String,
    pub url: Option<String>,
}