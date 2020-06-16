use std::path::Path;
use std::convert::AsRef;
use std::path::MAIN_SEPARATOR;

pub fn to_href_separator<P: AsRef<Path>>(p: P) -> String {
    return p.as_ref()
        .iter()
        .map(|c| c.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
}

pub fn to_path_separator(url: &str) -> String {
    return url
        .split("/")
        .collect::<Vec<_>>()
        .join(&MAIN_SEPARATOR.to_string());
}

pub fn trim_start_slash(url: &str) -> &str {
    url.trim_start_matches("/")
}

pub fn trim_end_slash(url: &str) -> &str {
    url.trim_end_matches("/")
}

pub fn trim_slash(url: &str) -> &str {
    trim_start_slash(trim_end_slash(url))
}

pub fn is_dir(url: &str) -> bool {
    url.ends_with("/") || !url.contains(".")
}
