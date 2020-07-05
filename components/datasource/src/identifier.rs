use std::path::Path;
use serde_json::Value;

pub trait DocumentIdentifier {
    fn identifier(&self, path: &Path, document: &Value) -> String;
}

pub struct FileNameIdentifier {}

impl DocumentIdentifier for FileNameIdentifier {
    fn identifier(&self, path: &Path, _document: &Value) -> String {
        if let Some(stem) = path.file_stem() {
            let name = stem.to_string_lossy().into_owned();
            return slug::slugify(&name)
        }
        return String::from("")
    }
}

