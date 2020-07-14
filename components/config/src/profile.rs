use std::fmt;
use std::path::PathBuf;
use std::convert::From;
use std::collections::HashMap;

use serde::{Deserialize, Serialize, Serializer};
use serde_with::skip_serializing_none;

use super::config;

static DEBUG: &str = "debug";
static RELEASE: &str = "release";

static DEVELOPMENT: &str = "development";
static PRODUCTION: &str = "production";

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(from = "String", untagged)]
pub enum ProfileName {
    Debug,
    Release,
    Custom(String),
}

impl Serialize for ProfileName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        match *self {
            ProfileName::Debug => {
                serializer.serialize_str(DEBUG)
            },
            ProfileName::Release => {
                serializer.serialize_str(RELEASE)
            },
            ProfileName::Custom(ref val) => {
                serializer.serialize_str(val)
            },
        }
    }
}

impl Default for ProfileName {
    fn default() -> Self {
        ProfileName::Debug
    }
}

impl From<String> for ProfileName {
    fn from(s: String) -> Self {
        if s == DEBUG {
            ProfileName::Debug
        } else if s == RELEASE {
            ProfileName::Release
        } else {
            ProfileName::Custom(s) 
        }
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ProfileName::Custom(ref val) => write!(f, "{}", val),
            ProfileName::Debug => write!(f, "{}", DEBUG),
            ProfileName::Release => write!(f, "{}", RELEASE),
        }
    }
}

impl ProfileName {
    pub fn get_node_env(&self, debug: Option<String>, release: Option<String>) -> String {
        match self {
            ProfileName::Debug => {
                if let Some(env) = debug {
                    return env;
                }
                return DEVELOPMENT.to_string();
            }
            ProfileName::Release => {
                if let Some(env) = release {
                    return env;
                }
                return PRODUCTION.to_string();
            }
            ProfileName::Custom(s) => return s.to_string(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "kebab-case")]
pub struct ProfileSettings {
    #[serde(skip)]
    pub name: ProfileName,

    pub source: PathBuf,
    pub target: PathBuf,

    pub types: Option<RenderTypes>,
    pub strict: Option<bool>,

    pub pages: Option<PathBuf>,
    pub assets: Option<PathBuf>,
    pub locales: Option<PathBuf>,
    pub includes: Option<PathBuf>,
    pub partials: Option<PathBuf>,
    pub data_sources: Option<PathBuf>,
    pub resources: Option<PathBuf>,
    pub layout: Option<PathBuf>,

    #[serde(skip)]
    pub follow_links: Option<bool>,

    // FIXME: refactor to HTML flag
    pub render: Option<Vec<String>>,

    pub max_depth: Option<usize>,
    pub profile: Option<String>,
    pub live: Option<bool>,
    pub release: Option<bool>,

    pub shortcodes: Option<bool>,
    pub use_layout: Option<bool>,

    pub rewrite_index: Option<bool>,
    pub include_index: Option<bool>,

    pub incremental: Option<bool>,
    pub pristine: Option<bool>,
    pub force: Option<bool>,
    // Collate page data when defined
    pub collate: Option<bool>,

    pub write_redirects: Option<bool>,

    // Base URL to strip when building links etc
    pub base: Option<String>,

    // Specific set of paths to build
    pub paths: Option<Vec<PathBuf>>,

    // A base URL to strip from links
    pub base_href: Option<String>,

    pub host: Option<String>,
    pub port: Option<u16>,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            name: Default::default(),

            source: PathBuf::from(config::SITE),
            target: PathBuf::from(config::BUILD),
            types: Some(Default::default()),
            strict: Some(true),

            pages: Some(PathBuf::from(config::PAGE_DATA)),
            assets: Some(PathBuf::from(config::ASSETS)),
            locales: Some(PathBuf::from(config::LOCALES)),
            includes: Some(PathBuf::from(config::INCLUDES)),
            partials: Some(PathBuf::from(config::PARTIALS)),
            data_sources: Some(PathBuf::from(config::DATASOURCES)),
            resources: Some(PathBuf::from(config::RESOURCES)),
            layout: Some(PathBuf::from(config::LAYOUT_HBS)),

            rewrite_index: None,
            follow_links: Some(true),
            render: None,

            shortcodes: None,

            max_depth: None,
            profile: None,
            host: Some(config::HOST.to_string()),
            port: Some(config::PORT),
            live: None,
            release: None,
            include_index: None,
            incremental: Some(false),
            pristine: Some(true),
            force: None,
            collate: Some(true),
            write_redirects: None,
            base: None,
            paths: None,
            use_layout: Some(true),
            base_href: None,
        }
    }
}

impl ProfileSettings {

    pub fn get_host(&self) -> String {
        if let Some(ref host) = self.host {
            host.clone() 
        } else {
            config::HOST.to_string()
        }
    }

    pub fn get_port(&self) -> u16 {
        if let Some(ref port) = self.port {
            port.clone()
        } else {
            config::PORT
        }
    }

    pub fn is_live(&self) -> bool {
        self.live.is_some() && self.live.unwrap()
    }

    pub fn is_release(&self) -> bool {
        self.release.is_some() && self.release.unwrap()
    }

    pub fn is_force(&self) -> bool {
        self.force.is_some() && self.force.unwrap()
    }

    pub fn is_incremental(&self) -> bool {
        self.incremental.is_some() && self.incremental.unwrap()
    }

    pub fn is_pristine(&self) -> bool {
        self.pristine.is_some() && self.pristine.unwrap()
    }

    pub fn should_collate(&self) -> bool {
        self.collate.is_some() && self.collate.unwrap()
    }

    pub fn should_use_layout(&self) -> bool {
        self.use_layout.is_some() && self.use_layout.unwrap()
    }

    pub fn should_include_index(&self) -> bool {
        self.include_index.is_some() && self.include_index.unwrap()
    }

    pub fn should_rewrite_index(&self) -> bool {
        self.rewrite_index.is_some() && self.rewrite_index.unwrap()
    }

    pub fn should_follow_links(&self) -> bool {
        self.follow_links.is_some() && self.follow_links.unwrap()
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeOptions {
    // The current language
    pub lang: String,
    // Project root
    pub project: PathBuf,
    // Root for the input source files
    pub source: PathBuf,
    // Root of the output
    pub output: PathBuf,
    // Target output directory including a build tag
    pub base: PathBuf,
    // Target output directory including a build tag and
    // a locale identifier when multilingual
    pub target: PathBuf,
    // The computed profile to use
    pub settings: ProfileSettings,
}

impl RuntimeOptions {
    pub fn get_page_data_path(&self) -> PathBuf {
        self.project.join(self.settings.pages.as_ref().unwrap())
    }

    pub fn get_layout_path(&self) -> PathBuf {
        self.source.join(self.settings.layout.as_ref().unwrap())
    }

    pub fn get_assets_path(&self) -> PathBuf {
        self.source.join(self.settings.assets.as_ref().unwrap())
    }

    pub fn get_includes_path(&self) -> PathBuf {
        self.source.join(self.settings.includes.as_ref().unwrap())
    }

    pub fn get_partials_path(&self) -> PathBuf {
        self.source.join(self.settings.partials.as_ref().unwrap())
    }

    pub fn get_data_sources_path(&self) -> PathBuf {
        self.source.join(self.settings.data_sources.as_ref().unwrap())
    }

    pub fn get_resources_path(&self) -> PathBuf {
        self.source.join(self.settings.resources.as_ref().unwrap())
    }

    pub fn get_locales(&self) -> PathBuf {
        self.source.join(self.settings.locales.as_ref().unwrap())
    }

}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenderTypes {
    #[serde(flatten)]
    pub types: HashMap<String, PageType>,
}

impl Default for RenderTypes {
    fn default() -> Self {
        let mut types: HashMap<String, PageType> = HashMap::new();
        types.insert(
            config::MD.to_string(), 
            PageType {
                markdown: Some(true),
                ext: Some(config::HTML.to_string()),
            });
        Self { types }
    }
}

impl RenderTypes {
    // Get list of file extensions to render
    pub fn render(&self) -> Vec<String> {
        self.types
            .keys()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
    }

    // Get the extension mapping
    pub fn map(&self) -> HashMap<String, String> {
        let mut map: HashMap<String, String> = HashMap::new();
        for (k, v) in self.types.iter() {
            if let Some(ref ext) = v.ext {
                map.insert(k.to_string(), ext.to_string());
            }
        }
        map
    }

    // Get list of extension to parse as markdown
    pub fn markdown(&self) -> Vec<String> {
        self.types
            .iter()
            .filter(|(_k, v)| v.markdown.is_some() && v.markdown.unwrap())
            .map(|(k, _v)| k.to_string())
            .collect::<Vec<_>>()
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PageType {
    // Map this page type to another extension
    pub ext: Option<String>,
    // Parse this page type as markdown
    pub markdown: Option<bool>,
}

impl Default for PageType {
    fn default() -> Self {
        Self {
            ext: None,
            markdown: None,
        }
    }
}
