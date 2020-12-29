use crate::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Extract locale identifier from a file name when possible.
pub fn extract_locale(
    file: &PathBuf,
    languages: &Vec<String>,
) -> (Option<String>, PathBuf) {
    //let project = self.updater.project();
    //let languages = project.locales.languages().alternate();
    //languages.foo();
    if let Some((lang, path)) =
        collator::get_locale_file_info(&file.as_path(), &languages)
    {
        return (Some(lang), path);
    }
    (None, file.to_path_buf())
}

pub(crate) fn relative_to<P: AsRef<Path>>(
    file: P,
    base: P,
    target: P,
) -> Result<PathBuf> {
    let f = file.as_ref().canonicalize()?;
    let b = base.as_ref().canonicalize()?;
    let t = target.as_ref().to_path_buf();
    Ok(t.join(f.strip_prefix(b)?))
}

/// Convert a path to it's canonical representation infallibly.
pub(crate) fn canonical<P: AsRef<Path>>(src: P) -> PathBuf {
    let file = src.as_ref().to_path_buf();
    if file.exists() {
        if let Ok(canonical) = file.canonicalize() {
            return canonical;
        }
    }
    file
}

/// Walk the parent directory so we can determine if a path
/// should be ignored using the standard .gitignore and .ignore
/// file comparisons.
///
/// This is inefficient because we have to walk all the entries
/// in the parent directory to determine if a file should be
/// ignored.
///
/// Ideally we could do this at a lower-level but the `ignore`
/// crate does not expose the `dir` module so we would need to
/// reproduce all of that functionality.
pub(crate) fn filter_ignores(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut results: Vec<PathBuf> = Vec::new();
    for path in paths {
        if let Some(parent) = path.parent() {
            for entry in WalkBuilder::new(parent)
                .max_depth(Some(1))
                .filter_entry(move |entry| entry.path() == path)
                .build()
            {
                match entry {
                    Ok(entry) => {
                        if entry.path().is_file() {
                            results.push(entry.path().to_path_buf())
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    results
}
