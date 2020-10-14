use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;

use chrono::prelude::*;
pub use jsonfeed::{Attachment, Author, Feed};

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::{serde_as, skip_serializing_none, DisplayFromStr};

use crate::{
    date::DateConfig, indexer::QueryList, script::ScriptAsset,
    style::StyleAsset, utils::toml_datetime::from_toml_datetime, Config, Error,
    Result, RuntimeOptions,
};

use self::{feed::FeedEntry, file_context::FileContext};

pub(crate) mod feed;
pub(crate) mod file_context;
pub(crate) mod menu;
pub(crate) mod paginate;

pub use paginate::{PageLink, PaginateInfo};

#[serde_as]
#[skip_serializing_none]
#[derive(Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CollatedPage<'a> {
    #[serde(flatten)]
    pub page: &'a Page,

    pub lang: &'a str,
    pub charset: &'a str,
    pub date: &'a Option<DateConfig>,

    // Paths referenced in a menu when MENU.md convention is used
    //  FIXME: use a better name for the main menu
    pub main: Vec<&'a String>,
    pub menus: HashMap<&'a String, Vec<&'a String>>,

    pub generator: &'a str,
    #[serde_as(as = "DisplayFromStr")]
    pub version: &'a Version,
}

impl<'a> CollatedPage<'a> {
    pub fn new(config: &'a Config, page: &'a Page, lang: &'a str) -> Self {
        Self {
            page,
            lang,
            charset: config.charset(),
            date: &config.date,
            main: Default::default(),
            menus: Default::default(),
            generator: crate::generator::id(),
            version: config.version(),
        }
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
    pub summary: Option<String>,
    pub keywords: Option<String>,
    pub meta: Option<HashMap<String, String>>,

    pub render: Option<bool>,
    pub rewrite_index: Option<bool>,
    pub draft: Option<bool>,
    pub standalone: Option<bool>,
    pub listing: Option<bool>,
    pub noindex: Option<bool>,
    pub print: Option<bool>,
    pub fallback: Option<bool>,

    pub authors: Option<Vec<Author>>,
    pub byline: Option<Vec<String>>,

    pub query: Option<QueryList>,

    pub layout: Option<String>,
    pub taxonomies: Option<HashMap<String, Vec<String>>>,

    pub scripts: Option<Vec<ScriptAsset>>,
    pub styles: Option<Vec<StyleAsset>>,
    pub permalink: Option<String>,

    // Custom values for feed entry
    pub entry: Option<FeedEntry>,

    // Menus keyed by name
    #[serde(skip_serializing)]
    pub menu: Option<menu::MenuConfig>,

    // Automatically assigned

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
    pub host: Option<String>,
    #[serde(skip_deserializing)]
    pub href: Option<String>,
    #[serde(skip_deserializing)]
    pub file: Option<FileContext>,
    #[serde(skip_deserializing)]
    pub canonical: Option<String>,
    #[serde(skip_deserializing)]
    pub paginate: Option<PaginateInfo>,
    #[serde(skip_deserializing)]
    pub feed: Option<Feed>,

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
            summary: None,
            keywords: None,
            meta: None,
            authors: None,
            byline: None,
            rewrite_index: None,
            render: Some(true),
            draft: None,
            standalone: None,
            listing: Some(true),
            noindex: None,
            print: None,
            fallback: None,
            query: None,
            layout: None,
            taxonomies: None,
            scripts: None,
            styles: None,
            permalink: None,
            entry: None,
            menu: None,

            created: None,
            updated: None,

            extra: Map::new(),

            host: None,
            href: None,
            file: None,
            canonical: None,
            paginate: None,
            feed: None,
        }
    }
}

impl Page {
    pub fn new(
        config: &Config,
        options: &RuntimeOptions,
        file: &PathBuf,
    ) -> Result<Self> {
        let mut page: Page = Default::default();

        let destination = options.destination().build(file)?;

        page.seal(config, options, &file, &destination, None)?;

        Ok(page)
    }

    // This should be a W3C Datetime string suitable for a
    // sitemap lastmod field.
    pub fn lastmod(&self) -> String {
        let file_ctx = self.file.as_ref().unwrap();
        file_ctx.modified.to_rfc3339()
    }

    pub fn get_template(&self) -> &PathBuf {
        let file_ctx = self.file.as_ref().unwrap();
        &file_ctx.template
    }

    pub fn is_draft(&self, options: &RuntimeOptions) -> bool {
        if options.settings.is_release() {
            return self.draft.is_some() && self.draft.unwrap();
        }
        false
    }

    pub fn seal(
        &mut self,
        config: &Config,
        options: &RuntimeOptions,
        source: &PathBuf,
        output: &PathBuf,
        template: Option<PathBuf>,
    ) -> Result<()> {
        self.host = Some(config.host.clone());

        let template = if let Some(template) = template {
            template
        } else {
            source.clone()
        };

        let mut file_context =
            FileContext::new(source.clone(), output.clone(), template);
        file_context.resolve_metadata()?;

        self.href =
            Some(options.absolute(&file_context.source, Default::default())?);

        // TODO: allow setting to control this behavior
        if self.updated.is_none() {
            self.updated = Some(file_context.modified.clone());
        }

        self.file = Some(file_context);
        self.canonical = Some(options.settings.get_host_url(config));

        Ok(())
    }

    pub fn compute(
        &mut self,
        config: &Config,
        _opts: &RuntimeOptions,
    ) -> Result<()> {
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
                        return Err(Error::NoAuthor(id.to_string()));
                    }
                } else {
                    return Err(Error::NoAuthor(id.to_string()));
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

        if let Some(summary) = other.summary.as_mut() {
            self.summary = Some(mem::take(summary));
        }

        if let Some(keywords) = other.keywords.as_mut() {
            self.keywords = Some(mem::take(keywords));
        }

        if let Some(meta) = other.meta.as_mut() {
            self.meta = Some(mem::take(meta));
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

        if let Some(noindex) = other.noindex.as_mut() {
            self.noindex = Some(mem::take(noindex));
        }

        if let Some(print) = other.print.as_mut() {
            self.print = Some(mem::take(print));
        }

        if let Some(fallback) = other.fallback.as_mut() {
            self.fallback = Some(mem::take(fallback));
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

        if let Some(taxonomies) = other.taxonomies.as_mut() {
            self.taxonomies = Some(mem::take(taxonomies));
        }

        if let Some(scripts) = other.scripts.as_mut() {
            self.scripts = Some(mem::take(scripts));
        }

        if let Some(styles) = other.styles.as_mut() {
            self.styles = Some(mem::take(styles));
        }

        if let Some(permalink) = other.permalink.as_mut() {
            self.permalink = Some(mem::take(permalink));
        }

        if let Some(entry) = other.entry.as_mut() {
            self.entry = Some(mem::take(entry));
        }

        self.created = other.created.clone();
        self.updated = other.updated.clone();

        if let Some(href) = other.href.as_mut() {
            self.href = Some(mem::take(href));
        }

        self.extra.append(&mut other.extra);
    }
}
