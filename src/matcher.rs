use std::path::PathBuf;
//use std::ffi::OsStr;
use regex::Regex;

pub struct FileMatcher {
    exclude: Option<Vec<Regex>>,
    layout: String,
    template: String,
}

#[derive(Debug)]
pub enum FileType {
    Ignored,
    Markdown,
    Html,
    Template,
    Private,
    Unknown,
}

const INDEX: &'static str = "index";
const THEME: &'static str = "theme";
const PARSE_EXTENSIONS:[&'static str; 3] = ["html", "hbs", "md"];

const MD: &'static str = ".md";
const HTML: &'static str = ".html";
const HBS: &'static str = ".hbs";
const TOML: &'static str = ".toml";

impl FileMatcher {
    pub fn new(exclude: Option<Vec<Regex>>, layout: String, template: String) -> Self {
        FileMatcher{exclude, layout, template}
    } 

    //pub fn is_layout(&self, file: &PathBuf) -> bool {
        //if let Some(nm) = file.file_name() {
            //if nm == OsStr::new(&self.layout) {
                //return true
            //} 
        //} 
        //false
    //}

    pub fn is_index(&self, file: &PathBuf) -> bool {
        if let Some(nm) = file.file_stem() {
            if nm == INDEX {
                return true
            } 
        } 
        false
    }

    pub fn get_index_stem(&self) -> &str {
        INDEX
    }

    pub fn has_parse_file(&self, file: &PathBuf) -> bool {
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
        root_theme.push(&self.template);
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
                    if nm == self.layout {
                        return FileType::Private
                    }else if nm == self.template {
                        return FileType::Private
                    } else if nm.ends_with(MD) {
                        return FileType::Markdown
                    } else if nm.ends_with(HTML) {
                        return FileType::Html
                    } else if nm.ends_with(HBS) {
                        return FileType::Template
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
