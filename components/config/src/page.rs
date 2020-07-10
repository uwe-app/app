use std::io;
use std::mem;
use std::path::{Path, PathBuf};

use chrono::prelude::*;

use serde::{Deserialize, Serialize, Deserializer};
use serde_json::{Map, Value};
use serde_with::skip_serializing_none;

use super::Error;
use super::link;
use super::config::{Config, ExtensionConfig};
use super::indexer::QueryList;

use crate::config::HTML;

static INDEX_STEM: &str = "index";

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

#[derive(Debug)]
pub struct FileInfo {
    // The root of the source files
    pub source: PathBuf,
    // The root of the build target
    pub target: PathBuf,
    // A source file path
    pub file: PathBuf,
}

#[derive(Debug)]
pub enum FileType {
    Markdown,
    Template,
    Unknown,
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

    pub authors: Option<Vec<Author>>,
    pub byline: Option<Vec<String>>,

    pub query: Option<QueryList>,

    pub layout: Option<PathBuf>,
    pub tags: Option<Vec<String>>,

    pub scripts: Option<Vec<String>>,
    pub styles: Option<Vec<String>>,

    #[serde(deserialize_with = "from_toml_datetime")]
    pub created: Option<DateTime<Utc>>,

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
            query: None,
            layout: None,
            tags: None,
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

    pub fn is_clean<P: AsRef<Path>>(file: P, extensions: &ExtensionConfig) -> bool {
        let target = file.as_ref().to_path_buf();
        let result = target.clone();
        return Page::rewrite_index_file(target, result, extensions).is_some();
    }

    pub fn is_page<P: AsRef<Path>>(p: P, extensions: &ExtensionConfig) -> bool {
        match Page::get_type(p, extensions) {
            FileType::Markdown | FileType::Template => {
                true
            },
            _ => false
        }
    }

    pub fn relative_to<P: AsRef<Path>>(file: P, base: P, target: P) -> Result<PathBuf, Error> {
        let f = file.as_ref().canonicalize()?;
        let b = base.as_ref().canonicalize()?;
        let mut t = target.as_ref().to_path_buf();
        let relative = f.strip_prefix(b)?;
        t.push(relative);
        Ok(t)
    }

    pub fn get_type<P: AsRef<Path>>(p: P, extensions: &ExtensionConfig) -> FileType {
        let file = p.as_ref();
        if let Some(ext) = file.extension() {
            let ext = ext.to_string_lossy().into_owned();
            if extensions.render.contains(&ext) {
                if extensions.markdown.contains(&ext) {
                    return FileType::Markdown;
                } else {
                    return FileType::Template;
                }
            }
        }
        FileType::Unknown
    }

    pub fn is_index<P: AsRef<Path>>(file: P) -> bool {
        if let Some(nm) = file.as_ref().file_stem() {
            if nm == INDEX_STEM {
                return true;
            }
        }
        false
    }

    fn has_parse_file_match<P: AsRef<Path>>(file: P, extensions: &ExtensionConfig) -> bool {
        let path = file.as_ref();
        let mut copy = path.to_path_buf();
        for ext in extensions.render.iter() {
            copy.set_extension(ext);
            if copy.exists() {
                return true;
            }
        }
        false
    }

    fn rewrite_index_file<P: AsRef<Path>>(file: P, result: P, extensions: &ExtensionConfig) -> Option<PathBuf> {
        let clean_target = file.as_ref();
        if !Page::is_index(&clean_target) {
            if let Some(parent) = clean_target.parent() {
                if let Some(stem) = clean_target.file_stem() {
                    let mut target = parent.to_path_buf();
                    target.push(stem);
                    target.push(INDEX_STEM);

                    if !Page::has_parse_file_match(&target, extensions) {
                        let clean_result = result.as_ref().clone();
                        if let Some(parent) = clean_result.parent() {
                            if let Some(stem) = clean_result.file_stem() {
                                let mut res = parent.to_path_buf();
                                res.push(stem);
                                res.push(INDEX_STEM);
                                res.set_extension(HTML);
                                return Some(res);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    // Build the direct destination file path.
    pub fn output<P: AsRef<Path>>(
        source: P,
        target: P,
        file: P,
        base_href: &Option<String>,
    ) -> Result<PathBuf, Error> {
        let pth = file.as_ref();

        // NOTE: When watching files we can get absolute
        // NOTE: paths passed for `file` even when `source`
        // NOTE: is relative. This handles that case by making
        // NOTE: the `source` absolute based on the current working
        // NOTE: directory.
        let mut src: PathBuf = source.as_ref().to_path_buf();
        if pth.is_absolute() && src.is_relative() {
            if let Ok(cwd) = std::env::current_dir() {
                src = cwd.clone();
                src.push(source.as_ref())
            }
        }

        let mut relative = pth.strip_prefix(src)?;

        if let Some(ref base) = base_href {
            if relative.starts_with(base) {
                relative = relative.strip_prefix(base)?;
            }
        }

        let result = target.as_ref().clone().join(relative);
        return Ok(result);
    }

    // Build the destination file path and update the file extension.
    pub fn destination<P: AsRef<Path>>(
        source: P,
        target: P,
        file: P,
        file_type: &FileType,
        extensions: &ExtensionConfig,
        rewrite_index: bool,
        base_href: &Option<String>,
    ) -> Result<PathBuf, Error> {

        let pth = file.as_ref().to_path_buf().clone();
        let result = Page::output(source, target, file, base_href);
        match result {
            Ok(mut result) => {
                match file_type {
                    FileType::Markdown | FileType::Template => {

                        if let Some(ext) = pth.extension() {
                            let ext = ext.to_string_lossy().into_owned();
                            for (k, v) in &extensions.map {
                                if ext == *k {
                                    result.set_extension(v);
                                    break;
                                }
                            }
                        }

                        if rewrite_index {
                            if let Some(res) = Page::rewrite_index_file(pth.as_path(), result.as_path(), extensions) {
                                result = res;
                            }
                        }
                    }
                    _ => {}
                }
                return Ok(result);
            }
            Err(e) => return Err(e),
        }
    }

    pub fn compute<P: AsRef<Path>>(&mut self, p: P, config: &Config) -> Result<(), Error> {

        self.href = Some(link::absolute(p.as_ref(), config, Default::default())?);

        let mut file_context = FileContext::new(p.as_ref().to_path_buf(), PathBuf::from(""));
        file_context.resolve_metadata()?;
        self.file = Some(file_context);

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

