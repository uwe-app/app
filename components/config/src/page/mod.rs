use std::collections::HashMap;
use std::collections::BTreeSet;
use std::mem;
use std::path::PathBuf;

use chrono::prelude::*;

use indexmap::IndexSet;

use jsonfeed::Feed;
use url::Url;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use serde_with::skip_serializing_none;

use crate::{
    href::UrlPath, indexer::QueryList, script::ScriptAsset, style::StyleAsset,
    tags::link::LinkTag, utils::toml_datetime::from_toml_datetime, Config,
    Result, RuntimeOptions,
};

use self::{feed::FeedEntry, file_context::FileContext};

pub(crate) mod author;
pub(crate) mod feed;
pub(crate) mod file_context;
pub(crate) mod paginate;

pub use author::Author;
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
    pub label: Option<String>,
    pub summary: Option<String>,
    pub image: Option<UrlPath>,
    pub meta: Option<HashMap<String, String>>,
    pub http_equiv: Option<HashMap<String, String>>,
    pub open_graph: Option<HashMap<String, String>>,

    pub render: Option<bool>,
    pub rewrite_index: Option<bool>,
    pub draft: Option<bool>,
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

    /// Ignore from directory listings.
    listing: Option<bool>,

    pub query: Option<QueryList>,

    pub layout: Option<String>,
    pub taxonomies: Option<HashMap<String, Vec<String>>>,

    #[serde(skip_serializing_if = "IndexSet::is_empty")]
    links: IndexSet<LinkTag>,

    #[serde(skip_serializing_if = "IndexSet::is_empty")]
    scripts: IndexSet<ScriptAsset>,

    #[serde(skip_serializing_if = "IndexSet::is_empty")]
    styles: IndexSet<StyleAsset>,

    #[serde(skip_serializing)]
    pub permalink: Option<UrlPath>,

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
    //#[serde(skip_deserializing)]
    //pub canonical: Option<String>,
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
            label: None,
            summary: None,
            image: None,
            meta: None,
            http_equiv: None,
            open_graph: None,
            authors: None,
            rewrite_index: None,
            render: Some(true),
            draft: None,
            standalone: None,
            listing: None,
            noindex: None,
            print: None,
            fallback: None,
            absolute: None,
            query: None,
            layout: None,
            taxonomies: None,
            links: IndexSet::new(),
            scripts: IndexSet::new(),
            styles: IndexSet::new(),
            permalink: None,
            entry: None,

            created: None,
            updated: None,

            extra: Map::new(),

            href: None,
            file: None,
            //canonical: None,
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

    /// Get the destination output file for this page.
    ///
    /// Will panic if the page has not been sealed.
    pub fn destination(&self) -> &PathBuf {
        &self.file.as_ref().unwrap().target
    }

    pub fn authors(&self) -> &Option<Vec<String>> {
        &self.authors
    }

    pub fn links(&self) -> &IndexSet<LinkTag> {
        &self.links
    }

    pub fn links_mut(&mut self) -> &mut IndexSet<LinkTag> {
        &mut self.links
    }

    pub fn set_links(&mut self, links: IndexSet<LinkTag>) {
        self.links = links;
    }

    pub fn styles(&self) -> &IndexSet<StyleAsset> {
        &self.styles
    }

    pub fn scripts(&self) -> &IndexSet<ScriptAsset> {
        &self.scripts
    }

    pub fn scripts_mut(&mut self) -> &mut IndexSet<ScriptAsset> {
        &mut self.scripts
    }

    pub fn set_scripts(&mut self, scripts: IndexSet<ScriptAsset>) {
        self.scripts = scripts;
    }

    /// Whether this page can be included in a directory listing.
    pub fn is_listable(&self) -> bool {
        self.listing.is_none()
            || self.listing.is_some() && self.listing.unwrap()
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

    /// Get an absolute permalink for the page; will panic if `href` is not set.
    pub fn permalink(
        &self,
        config: &Config,
        options: &RuntimeOptions,
    ) -> Result<Url> {
        let website = options.settings.get_host_url(config)?;
        if let Some(ref permalink) = self.permalink {
            Ok(website.join(permalink.as_str())?)
        } else {
            Ok(website.join(self.href.as_ref().unwrap())?)
        }
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

        let mut file_context = FileContext::new(
            config.project(),
            source.clone(),
            output.clone(),
            template,
        );
        file_context.resolve_metadata()?;

        let href =
            options.absolute(&file_context.source, Default::default())?;

        // TODO: allow setting to control this behavior
        if self.updated.is_none() {
            self.updated = Some(file_context.modified.clone());
        }

        // FIXME: add page page for canonical and
        // FIXME: set host/domain/website for the page data (#252)

        let website = options.settings.get_host_url(config)?;
        let mut canonical = website.join(&href)?;

        let og = self.open_graph.get_or_insert(Default::default());
        og.insert(crate::OG_URL.to_string(), canonical.clone().to_string());

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
            let img = website.join(image.as_str().trim_start_matches("/"))?;
            og.entry(crate::OG_IMAGE.to_string())
                .or_insert(img.to_string());
        }

        self.file = Some(file_context);
        self.href = Some(href);

        if let Some(ref permalink) = self.permalink {
            let bookmark = website.join(permalink.as_str())?;
            self.links.insert(LinkTag::new_bookmark(bookmark.to_string()));
        }

        self.links
            .insert(LinkTag::new_canonical(canonical.to_string()));

        //self.canonical = Some(canonical);

        Ok(())
    }

    /// Compute is called after the loaded data inheritance has been handled
    /// and can be used to finalize default values for a page.
    pub fn compute(
        &mut self,
        _config: &Config,
        _opts: &RuntimeOptions,
    ) -> Result<()> {
        // Convert link labels to lower case by default so that
        // they fit better in long form natural language content.
        //
        // We found ourselves often assigning a label to the lower
        // case version of the title so this saves a lot of repetition.
        if let None = self.label {
            if let Some(ref title) = self.title {
                self.label = Some(title.to_lowercase());
            }
        }

        Ok(())
    }

    pub fn append(&mut self, other: &mut Self) {
        // NOTE: handle lists like this so precedence is correct

        //other.links.append(&mut self.links);
        //self.links = mem::take(&mut other.links);

        {
            let mut temp = other.links.clone();
            for link in self.links.drain(..) {
                temp.insert(link);
            }
            self.links = temp;
        }

        {
            let mut temp = other.scripts.clone();
            for script in self.scripts.drain(..) {
                temp.insert(script);
            }
            self.scripts = temp;
        }

        // NOTE: styles should have been converted to links
        // NOTE: so this may be unnecessary?
        self.styles = mem::take(&mut other.styles);

        if let Some(title) = other.title.as_mut() {
            self.title = Some(mem::take(title));
        }

        if let Some(description) = other.description.as_mut() {
            self.description = Some(mem::take(description));
        }

        if let Some(label) = other.label.as_mut() {
            self.label = Some(mem::take(label));
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
