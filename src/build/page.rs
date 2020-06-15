use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use std::mem;

use serde_with::skip_serializing_none;
use serde_json::{Map, Value};

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Page {
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub author: Option<Author>,
    pub clean: Option<bool>,
    pub draft: Option<bool>,
    pub standalone: Option<bool>,
    pub query: Option<Value>,
    pub layout: Option<PathBuf>,

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
