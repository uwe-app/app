use std::path::Path;

use utils;

pub fn write<P: AsRef<Path>>(location: &str, target: P) -> std::io::Result<()> {
    let mut content = String::from("<!doctype html>");
    let body = format!(
        "<body onload=\"document.location.replace('{}');\"></body>",
        location
    );
    let meta = format!(
        "<noscript><meta http-equiv=\"refresh\" content=\"0; {}\"></noscript>",
        location
    );
    content.push_str("<html>");
    content.push_str("<head>");
    content.push_str(&meta);
    content.push_str("</head>");
    content.push_str(&body);
    content.push_str("</html>");
    utils::fs::write_string(target, content)
}