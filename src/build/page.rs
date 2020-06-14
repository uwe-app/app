use serde::{Serialize, Deserialize};
use std::mem;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Page {
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub author: Option<Author>,
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

    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Author {
    pub name: String,
    pub url: Option<String>,
}
