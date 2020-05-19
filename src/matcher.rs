use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;

use super::Options;

pub struct FileMatcher<'a> {
    options: &'a Options,
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

impl<'a> FileMatcher<'a> {
    
    pub fn new(options: &'a Options) -> Self {
        FileMatcher{options} 
    }

    pub fn clean<P: AsRef<Path>>(&self, file: P, result: P) -> Option<PathBuf> {
        let clean_target = file.as_ref().clone();
        if !self.is_index(&clean_target) {
            if let Some(parent) = clean_target.parent() {
                if let Some(stem) = clean_target.file_stem() {
                    let mut target = parent.to_path_buf();
                    target.push(stem);
                    target.push(INDEX);

                    if !self.has_parse_file(&target) {
                        let clean_result = result.as_ref().clone();
                        if let Some(parent) = clean_result.parent() {
                            if let Some(stem) = clean_result.file_stem() {
                                let mut res = parent.to_path_buf();
                                res.push(stem);
                                res.push(INDEX);
                                res.set_extension("html");
                                return Some(res)
                            }
                        }
                    }
                }
            }

        }
        None
    }

    // Build the destination file path.
    pub fn destination(
        &self,
        file: &PathBuf,
        file_type: &FileType,
        clean: bool) -> PathBuf {

        let relative = file.strip_prefix(&self.options.source);
        match relative {
            Ok(relative) => {
                let mut result = self.options.target.clone().join(relative);
                match file_type {
                    FileType::Markdown | FileType::Html => {
                        result.set_extension("html");
                        if clean {
                            if let Some(res) = self.clean(file, &result) {
                                result = res;
                            }
                        }
                    },
                    _ => {}
                }
                result
            },
            Err(e) => panic!(e),
        }
    }

    pub fn is_index<P: AsRef<Path>>(&self, file: P) -> bool {
        if let Some(nm) = file.as_ref().file_stem() {
            if nm == INDEX {
                return true
            } 
        } 
        false
    }

    pub fn has_parse_file<P: AsRef<Path>>(&self, file: P) -> bool {
        let path = file.as_ref();
        let mut copy = path.to_path_buf();
        for ext in PARSE_EXTENSIONS.iter() {
            copy.set_extension(ext);
            if copy.exists() {
                return true; 
            }
        }
        false
    }

    pub fn is_excluded<P: AsRef<Path>>(&self, file: P) -> bool {
        let path = file.as_ref();
        if let Some(list) = &self.options.exclude {
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

    pub fn get_theme_dir<P: AsRef<Path>>(&self, base: P) -> PathBuf {
        let mut root_theme = base.as_ref().to_path_buf();
        root_theme.push(&self.options.template);
        root_theme.push(THEME);
        root_theme
    }

    pub fn is_theme<P: AsRef<Path>>(&self, base: P, file: P) -> bool {
        let root_theme = self.get_theme_dir(base);
        if &root_theme == file.as_ref() {
            return true
        }
        false
    }

    pub fn get_type<P: AsRef<Path>>(&self, file: P) -> FileType {
        // Explicitly excluded files take precedence
        if self.is_excluded(&file) {
            return FileType::Ignored
        }
        
        let name = file.as_ref().file_name();
        match name {
            Some(nm) => {
                if let Some(nm) = nm.to_str() {
                    if nm == self.options.layout || nm.starts_with(".") {
                        return FileType::Private
                    }else if nm == self.options.template {
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
