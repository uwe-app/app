use std::path::Path;
use log::info;
use std::collections::HashMap;
use std::fs;

use crate::{Error, Result, INDEX_HTML};

use warp::http::Uri;

static MAX_REDIRECTS: usize = 4;

fn write_file<P: AsRef<Path>>(location: &str, target: P) -> std::io::Result<()> {
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

pub fn write() -> Result<()> {
    let runtime = runtime::runtime().read().unwrap();

    if let Some(ref redirect) = runtime.config.redirect {
        for (k, v) in redirect {
            // Strip the trailing slash so it is not treated
            // as an absolute path on UNIX
            let key = k.trim_start_matches("/");

            let mut buf = runtime.options.base.clone();
            buf.push(utils::url::to_path_separator(key));
            if k.ends_with("/") {
                buf.push(INDEX_HTML);
            }
            if buf.exists() {
                return Err(Error::RedirectFileExists(buf));
            }

            let short = buf.strip_prefix(&runtime.options.base)?;
            info!("{} -> {} as {}", &k, &v, short.display());
            if let Some(ref parent) = buf.parent() {
                fs::create_dir_all(parent)?;
            }
            write_file(&v, &buf)?;
        }
    }
    Ok(())
}

pub fn collect(items: &HashMap<String, String>) -> Result<HashMap<String, Uri>> {
    let mut map: HashMap<String, Uri> = HashMap::new();
    for (k, v) in items {
        map.insert(k.clone(), v.as_str().parse::<Uri>()?);
    }
    Ok(map)
}

pub fn validate(map: &HashMap<String, String>) -> Result<()> {
    for (k, v) in map {
        let mut stack: Vec<String> = Vec::new();
        validate_redirect(k, v, map, &mut stack)?;
    }
    Ok(())
}

// FIXME: improve this redirect validation logic to handle
// FIXME: trailing slashes on sources and targets better

fn validate_redirect<S: AsRef<str>>(
    k: S,
    v: S,
    map: &HashMap<String, String>,
    stack: &mut Vec<String>,
) -> Result<()> {
    if stack.len() >= MAX_REDIRECTS {
        return Err(Error::TooManyRedirects(MAX_REDIRECTS));
    }

    let mut key = k.as_ref().to_string().clone();
    key = key.trim_end_matches("/").to_string();

    if stack.contains(&key) {
        return Err(Error::CyclicRedirect {stack: stack.join(" <-> "), key: key.clone()});
    }

    stack.push(key);

    // Check raw value first
    if let Some(value) = map.get(v.as_ref()) {
        return validate_redirect(v.as_ref(), value, map, stack);
    }

    // Try with a trailing slash removed
    let mut val_key = v.as_ref().to_string();
    val_key = val_key.trim_end_matches("/").to_string();
    if let Some(value) = map.get(&val_key) {
        return validate_redirect(&val_key, value, map, stack);
    }

    Ok(())
}
