use std::path::PathBuf;
use regex::Regex;

pub struct FileMatcher {
    exclude: Option<Vec<Regex>>,
}

#[derive(Debug)]
pub enum FileType {
    Ignored,
    Markdown,
    Html,
    Handlebars,
    Private,
    Unknown,
}

const THEME: &'static str = "theme";
const TEMPLATE_NAME: &'static str = "hypertext.hbs";
const PARSE_EXTENSIONS:[&'static str; 3] = ["html", "hbs", "md"];

const MD: &'static str = ".md";
const HTML: &'static str = ".html";
const HBS: &'static str = ".hbs";
const TOML: &'static str = ".toml";

impl FileMatcher {
    pub fn new(exclude: Option<Vec<Regex>>) -> Self {
        FileMatcher{exclude}
    } 

    fn has_parse_file(&self, file: &PathBuf) -> bool {
        let mut copy = file.clone();
        for ext in PARSE_EXTENSIONS.iter() {
            copy.set_extension(ext);
            if copy.exists() {
                return true; 
            }
        }
        false
    }

    pub fn is_excluded(&self, path: &PathBuf) -> bool {
        if let Some(list) = &self.exclude {
            for ptn in list {
                if let Some(s) = path.to_str() {
                    if ptn.is_match(s) {
                        return true
                    }
                }
            }
        }
        false
    }

    pub fn get_theme_dir(&self, base: &PathBuf) -> PathBuf {
        let mut root_theme = base.clone();
        root_theme.push(THEME);
        root_theme
    }

    pub fn is_theme(&self, base: &PathBuf, file: &PathBuf) -> bool {
        let root_theme = self.get_theme_dir(base);
        if &root_theme == file {
            return true
        }
        false
    }

    pub fn get_type(&self, file: &PathBuf) -> FileType {
        // Explicitly excluded files take precedence
        if self.is_excluded(&file) {
            return FileType::Ignored
        }

        let name = file.file_name();
        match name {
            Some(nm) => {
                if let Some(nm) = nm.to_str() {
                    if nm == TEMPLATE_NAME {
                        return FileType::Private
                    }else if nm == THEME {
                        return FileType::Private
                    } else if nm.ends_with(MD) {
                        return FileType::Markdown
                    } else if nm.ends_with(HTML) {
                        return FileType::Html
                    } else if nm.ends_with(HBS) {
                        return FileType::Handlebars
                    } else if nm.ends_with(TOML) && self.has_parse_file(file) {
                        return FileType::Private
                    }
                }
            },
            _ => {}
        }
        FileType::Unknown
    }

}
