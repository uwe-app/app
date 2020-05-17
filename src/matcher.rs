use std::path::PathBuf;
use regex::Regex;

pub struct FileMatcher {
    ignore: Option<Vec<Regex>>,
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

const TOML: &'static str = ".toml";

const PARSE_EXTENSIONS:[&'static str; 3] = ["html", "hbs", "md"];

impl FileMatcher {
    pub fn new(ignore: Option<Vec<Regex>>) -> Self {
        FileMatcher{ignore}
    } 

    fn is_ignored(&self, path: &PathBuf) -> bool {
        if let Some(list) = &self.ignore {
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

    pub fn get_type(&self, file: &PathBuf) -> FileType {
        // Explicitly ignored files take precedence
        if self.is_ignored(&file) {
            return FileType::Ignored
        }

        let name = file.file_name();
        match name {
            Some(nm) => {
                if let Some(nm) = nm.to_str() {
                    if nm == "hypertext.hbs" {
                        return FileType::Private
                    } else if nm.ends_with(".md") {
                        return FileType::Markdown
                    } else if nm.ends_with(".html") {
                        return FileType::Html
                    } else if nm.ends_with(".hbs") {
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
