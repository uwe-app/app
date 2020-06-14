use std::path::Path;

use log::info;

use crate::{utils, Error, INDEX_HTML};

pub fn write<P: AsRef<Path>>(lang: &str, target: P) -> Result<(), Error> {
    let mut dest = target.as_ref().to_path_buf();
    dest.push(INDEX_HTML);
    info!("Redirect {} -> {}", dest.display(), &lang);

    let mut content = String::from("<!doctype html>");
    let body = format!("<body onload=\"document.location.replace('/{}/');\"></body>", lang);
    let meta = format!("<noscript><meta http-equiv=\"refresh\" content=\"0; /{}/\"></noscript>", lang);
    content.push_str("<html>");
    content.push_str("<head>");
    content.push_str(&meta);
    content.push_str("</head>");
    content.push_str(&body);
    content.push_str("</html>");

    utils::write_string(dest, content).map_err(Error::from)
}
