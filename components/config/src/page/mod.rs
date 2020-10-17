use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;

use chrono::prelude::*;

pub use jsonfeed::{Attachment, Author, Feed};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::skip_serializing_none;

use crate::{
    href::UrlPath, indexer::QueryList, script::ScriptAsset, style::StyleAsset,
    utils::toml_datetime::from_toml_datetime, Config, Result,
    RuntimeOptions,
};

use self::{feed::FeedEntry, file_context::FileContext};

pub(crate) mod feed;
pub(crate) mod file_context;
pub(crate) mod paginate;

pub use paginate::{PageLink, PaginateInfo};

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
    pub image: Option<UrlPath>,
    pub meta: Option<HashMap<String, String>>,
    pub open_graph: Option<HashMap<String, String>>,

    pub render: Option<bool>,
    pub rewrite_index: Option<bool>,
    pub draft: Option<bool>,
    pub listing: Option<bool>,
    pub fallback: Option<bool>,

    /// Do not render a layout for this page.
    pub standalone: Option<bool>,

    /// Flag to indicate this page is intended for print media.
    print: Option<bool>,

    /// Instruct robots not to index this page.
    noindex: Option<bool>,

    /// Instruction for helpers to make links absolute.
    absolute: Option<bool>,

    authors: Option<Vec<String>>,

    pub query: Option<QueryList>,

    pub layout: Option<String>,
    pub taxonomies: Option<HashMap<String, Vec<String>>>,

    pub scripts: Option<Vec<ScriptAsset>>,
    pub styles: Option<Vec<StyleAsset>>,
    pub permalink: Option<String>,

    // Custom values for feed entry
    pub entry: Option<FeedEntry>,

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
    pub href: Option<String>,
    #[serde(skip_deserializing)]
    pub file: Option<FileContext>,
    #[serde(skip_deserializing)]
    pub canonical: Option<String>,
    #[serde(skip_deserializing)]
    pub paginate: Option<PaginateInfo>,
    #[serde(skip_deserializing)]
    pub feed: Option<Feed>,

    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            summary: None,
            image: None,
            meta: None,
            open_graph: None,
            authors: None,
            rewrite_index: None,
            render: Some(true),
            draft: None,
            standalone: None,
            listing: Some(true),
            noindex: None,
            print: None,
            fallback: None,
            absolute: None,
            query: None,
            layout: None,
            taxonomies: None,
            scripts: None,
            styles: None,
            permalink: None,
            entry: None,

            created: None,
            updated: None,

            extra: Map::new(),

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

    pub fn authors(&self) -> &Option<Vec<String>> {
        &self.authors
    }

    //pub fn is_standalone(&self) -> bool {
        //return self.standalone.is_some() && self.standalone.unwrap()
    //}

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
        let template = if let Some(template) = template {
            template
        } else {
            source.clone()
        };

        let mut file_context =
            FileContext::new(source.clone(), output.clone(), template);
        file_context.resolve_metadata()?;

        let href =
            options.absolute(&file_context.source, Default::default())?;

        // TODO: allow setting to control this behavior
        if self.updated.is_none() {
            self.updated = Some(file_context.modified.clone());
        }

        // FIXME: add page page for canonical and
        // FIXME: set host/domain/website for the page data (#252)

        let website = options.settings.get_host_url(config);

        let mut canonical = website.clone();
        canonical.push_str(href.trim_start_matches("/"));

        let og = self.open_graph.get_or_insert(Default::default());
        og.insert(crate::OG_URL.to_string(), canonical.clone());

        og.entry(crate::OG_TYPE.to_string())
            .or_insert(crate::OG_WEBSITE.to_string());

        if let Some(ref title) = self.title {
            og.entry(crate::OG_TITLE.to_string())
                .or_insert(title.clone());
        }
        if let Some(ref description) = self.description {
            og.entry(crate::OG_DESCRIPTION.to_string())
                .or_insert(description.clone());
        }
        if let Some(ref image) = self.image {
            let mut img = website.clone();
            img.push_str(image.as_str().trim_start_matches("/"));
            og.entry(crate::OG_IMAGE.to_string()).or_insert(img);
        }

        self.file = Some(file_context);
        self.href = Some(href);
        self.canonical = Some(canonical);

        Ok(())
    }

    pub fn compute(
        &mut self,
        config: &Config,
        _opts: &RuntimeOptions,
    ) -> Result<()> {

        //let mut authors_list = if let Some(ref author) = self.authors {
            //author.clone()
        //} else {
            //Vec::new()
        //};

        // TODO: finalize this page data after computation
        // TODO: build dynamic sort keys like date tuple (year, month, day) etc.

        /*
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
        */

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

        if let Some(image) = other.image.as_mut() {
            self.image = Some(mem::take(image));
        }

        if let Some(meta) = other.meta.as_mut() {
            self.meta = Some(mem::take(meta));
        }

        if let Some(open_graph) = other.open_graph.as_mut() {
            self.open_graph = Some(mem::take(open_graph));
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

        if let Some(absolute) = other.absolute.as_mut() {
            self.absolute = Some(mem::take(absolute));
        }

        if let Some(fallback) = other.fallback.as_mut() {
            self.fallback = Some(mem::take(fallback));
        }

        if let Some(authors) = other.authors.as_mut() {
            self.authors = Some(mem::take(authors));
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
