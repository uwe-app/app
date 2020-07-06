use std::path::Path;
use serde_json::Value;

// TODO: support extracting an `id` field from the document

pub enum Strategy {
    FileName,
    Count,
}

pub struct ComputeIdentifier {}

impl ComputeIdentifier{
    pub fn id(
        strategy: &Strategy,
        path: &Path,
        _document: &Value,
        count: &usize) -> String {

        match strategy {
            Strategy::FileName => {
                if let Some(stem) = path.file_stem() {
                    let name = stem.to_string_lossy().into_owned();
                    return slug::slugify(&name)
                }
            },
            _ => {
                return format!("{}", count);
            }
        }
        format!("{}", count)
    }
}

