use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::RwLock;

use once_cell::sync::OnceCell;

use config::syntax::SyntaxConfig;

use syntect::dumps::from_reader;
use syntect::parsing::SyntaxReference;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::html::ClassedHTMLGenerator;

//use syntect::html::css_for_theme;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unknown theme '{0}', supported values are: {1}")]
    UnknownTheme(String, String),

    #[error("Could not load cached syntax set {0}")]
    SyntaxSetLoad(PathBuf),

    #[error("Could not parse cached syntax set")]
    SyntaxSetParse,

    #[error("Could not load cached theme set {0}")]
    ThemeSetLoad(PathBuf),

    #[error("Could not parse cached theme set")]
    ThemeSetParse,
}

type Result<T> = std::result::Result<T, Error>;

mod inline;

#[derive(Debug)]
struct HighlightAssets {
    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
}

impl Default for HighlightAssets {
    fn default() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }
}

impl HighlightAssets {
    pub fn from_cache(syntax_bin: &PathBuf, themes_bin: &PathBuf) -> Result<Self> {
        let syntax_set_file = File::open(syntax_bin)
            .map_err(|_e| Error::SyntaxSetLoad(syntax_bin.clone()))?;
        let syntax_set: SyntaxSet = from_reader(BufReader::new(syntax_set_file))
            .map_err(|_e| Error::SyntaxSetParse)?;
        let theme_set_file = File::open(themes_bin)
            .map_err(|_e| Error::ThemeSetLoad(themes_bin.clone()))?;
        let theme_set: ThemeSet = from_reader(BufReader::new(theme_set_file))
            .map_err(|_e| Error::ThemeSetParse)?;
        Ok(Self{ syntax_set, theme_set })
    }
}

fn initialized() -> &'static RwLock<bool> {
    static INSTANCE: OnceCell<RwLock<bool>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        RwLock::new(false)
    })
}

pub fn conf(conf: Option<SyntaxConfig>) -> &'static SyntaxConfig {
    static INSTANCE: OnceCell<SyntaxConfig> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        if let Some(conf) = conf {
            conf    
        } else {
            Default::default()
        }
    })
}

fn assets(assets: Option<HighlightAssets>) -> &'static HighlightAssets {
    static INSTANCE: OnceCell<HighlightAssets> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        if let Some(assets) = assets {
            assets
        } else {
            Default::default()
        }
    })
}

fn lookup() -> &'static HashMap<&'static str, &'static str> {
    static INSTANCE: OnceCell<HashMap<&'static str, &'static str>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut lang_lookup = HashMap::new();
        lang_lookup.insert("rust", "rs");
        lang_lookup.insert("handlebars", "hbs");

        // Add custom mappings from the config
        let map = conf(None).map.as_ref().unwrap();
        for (k, v) in map.iter() {
            lang_lookup.entry(k).or_insert(&v);
        }

        lang_lookup
    })
}

pub fn highlight<'a>(value: &str, syntax: &'a SyntaxReference) -> String {

    let config = conf(None);
    let assets = assets(None);
    let ps = &assets.syntax_set;

    if config.is_inline() {
        let ts = &assets.theme_set;

        return inline::highlighted_html_for_string(
            value,
            ps,
            syntax,
            &ts.themes[config.theme()]);
    }

        //
        //
    //println!("{}", css_for_theme(&ts.themes["base16-ocean.dark"]));
    //println!("{}", &value);

    let mut html_generator = ClassedHTMLGenerator::new(syntax, ps);
    for line in value.lines() {
        html_generator.parse_html_for_line(&line);
    }
    html_generator.finalize()
}

pub fn find<'a>(language: &str) -> Option<&'a SyntaxReference> {
    let assets = assets(None);
    let ps = &assets.syntax_set;
    let ll = lookup();
    
    if let Some(syntax) = ps.find_syntax_by_extension(language) {
        return Some(syntax)
    } else {
        if let Some(lang_ext) = ll.get(language) {
            if let Some(syntax) = ps.find_syntax_by_extension(lang_ext) {
                return Some(syntax)
            }
        }
    }

    None
}

// Perform the initial setup for syntax highlighting.
//
// This is expensive so should only be called when syntax 
// highlighting is enabled for a profile.
pub fn setup(syntax_dir: &PathBuf, config: &SyntaxConfig) -> Result<()> {

    {
        let is_setup = initialized().read().unwrap();
        if *is_setup {
            return Ok(())
        }
    }

    let syntax_bin = syntax_dir.join("binary/syntaxes.bin");
    let themes_bin = syntax_dir.join("binary/themes.bin");
    let assets_cache = HighlightAssets::from_cache(&syntax_bin, &themes_bin)?;

    // Store the configuration
    let conf = conf(Some(config.clone()));

    // Extract the bundled syntaxes and themes
    let assets = assets(Some(assets_cache));
    let ts = &assets.theme_set;

    if !ts.themes.contains_key(conf.theme()) {
        let supported = ts.themes
            .keys()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        return Err(Error::UnknownTheme(conf.theme().to_string(), supported)) 
    }

    let mut flag = initialized().write().unwrap();
    *flag = true;

    Ok(())
}
