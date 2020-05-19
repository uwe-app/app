use std::path::Path;
use std::convert::AsRef;

use inflector::Inflector;

const INDEX_STEM: &'static str = "index";

// Convert a file name to title case
pub fn file_auto_title<P : AsRef<Path>>(input: P) -> Option<String> {
    let i = input.as_ref();
    if let Some(nm) = i.file_stem() {
        // If the file is an index file, try to get the name 
        // from a parent directory
        if nm == INDEX_STEM {
            if let Some(p) = i.parent() {
                return file_auto_title(&p.to_path_buf());
            }
        } else {
            let auto = nm.to_str().unwrap().to_string();
            let capitalized = auto.to_title_case();
            return Some(capitalized)
        }

    }
    None
}
