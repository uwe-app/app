use std::collections::HashMap;

use once_cell::sync::OnceCell;

use config::syntax::SyntaxConfig;

use syntect::parsing::SyntaxReference;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::html::ClassedHTMLGenerator;

//use syntect::html::css_for_theme;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unknown theme '{0}', supported values are: {1}")]
    UnknownTheme(String, String)
}

type Result<T> = std::result::Result<T, Error>;

mod inline;

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

pub fn syntaxes() -> &'static SyntaxSet {
    static INSTANCE: OnceCell<SyntaxSet> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        SyntaxSet::load_defaults_newlines()
    })
}

pub fn themes() -> &'static ThemeSet {
    static INSTANCE: OnceCell<ThemeSet> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        ThemeSet::load_defaults()
    })
}

pub fn lookup() -> &'static HashMap<&'static str, &'static str> {
    static INSTANCE: OnceCell<HashMap<&'static str, &'static str>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut lang_lookup: HashMap<&'static str, &'static str> = HashMap::new();
        lang_lookup.insert("rust", "rs");
        //lang_lookup.insert("toml", "ini");
        lang_lookup
    })
}

pub fn highlight<'a>(value: &str, syntax: &'a SyntaxReference) -> String {

    let config = conf(None);
    let ps = syntaxes();

    if config.is_inline() {
        let ts = themes();
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
    let ps = syntaxes();
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
pub fn setup(config: &SyntaxConfig) -> Result<()> {
    // Store the configuration
    let conf = conf(Some(config.clone()));

    // Extract the bundled syntaxes and themes
    let _ = syntaxes();
    let ts = themes();

    if !ts.themes.contains_key(conf.theme()) {
        let supported = ts.themes
            .keys()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        return Err(Error::UnknownTheme(conf.theme().to_string(), supported)) 
    }

    Ok(())
}
