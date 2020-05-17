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
    Template,
    Unknown,
}

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

    pub fn get_type(&self, file: &PathBuf) -> FileType {
        // Explicitly ignored files take precedence
        if self.is_ignored(&file) {
            return FileType::Ignored
        }

        let name = file.file_name();
        match name {
            Some(nm) => {
                if let Some(nm) = nm.to_str() {
                    if nm.ends_with(".md") || nm.ends_with(".markdown") {
                        return FileType::Markdown
                    } else if nm.ends_with(".htm") || nm.ends_with(".html") {
                        return FileType::Html
                    } else if nm == "hypertext.hbs" {
                        return FileType::Template
                    } else if nm.ends_with(".hbs") {
                        return FileType::Handlebars
                    }
                }
            },
            _ => {}
        }
        FileType::Unknown
    }

}
