use std::path::{Path, PathBuf};

use config::{LocaleMap, LocaleName};

/// Extract a locale identifier from a file path and return
/// a new normalized path without the locale identifier.
pub fn get_locale_file_info(
    path: &Path,
    languages: &Vec<&str>,
) -> Option<(LocaleName, PathBuf)> {
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().unwrap();
        if let Some(stem) = path.file_stem() {
            let stem = stem.to_str().unwrap();
            // Verify the stem locale id is recognized
            if is_locale_stem(languages, stem) {
                // Rewrite the file path without the locale id
                let stem_path = Path::new(stem);
                let locale_id =
                    stem_path.extension().unwrap().to_str().unwrap();
                let parent_stem =
                    stem_path.file_stem().unwrap().to_str().unwrap();
                let fallback_name = format!("{}.{}", parent_stem, ext);
                let fallback = path.parent().unwrap().join(&fallback_name);

                return Some((locale_id.to_string(), fallback));
            }
        }
    }

    None
}

/// Get the target directory depending upon whether multiple locales
/// are available.
pub(crate) fn get_locale_target(
    lang: &str,
    base: &PathBuf,
    locales: &LocaleMap,
) -> PathBuf {
    if locales.multi {
        base.join(lang)
    } else {
        base.clone()
    }
}

fn is_locale_stem(names: &Vec<&str>, stem: &str) -> bool {
    for name in names {
        let ext = format!(".{}", name);
        if stem.ends_with(&ext) {
            return true;
        }
    }
    false
}
