use log::info;
use std::fs;
use std::collections::HashMap;

use super::context::Context;
use crate::content::redirect;
use crate::{utils, Result, Error, INDEX_HTML};

static MAX_REDIRECTS: i32 = 4;

pub fn write(context: &Context) -> Result<()> {
    if let Some(ref redirect) = context.config.redirect {
        for (k, v) in redirect {
            // Strip the trailing slash so it is not treated
            // as an absolute path on UNIX
            let key = k.trim_start_matches("/");

            let mut buf = context.options.base.clone();
            buf.push(utils::url::to_path_separator(key));
            if k.ends_with("/") {
                buf.push(INDEX_HTML);
            }
            if buf.exists() {
                return Err(Error::new(format!(
                    "Redirect file '{}' exists",
                    buf.display()
                )));
            }

            let short = buf.strip_prefix(&context.options.base)?;
            info!("{} -> {} as {}", &k, &v, short.display());
            if let Some(ref parent) = buf.parent() {
                fs::create_dir_all(parent)?;
            }
            redirect::write(&v, &buf)?;
        }
    }
    Ok(())
}

pub fn validate(map: &HashMap<String, String>) -> Result<()> {
    for (k, v) in map {
        validate_redirect(k, v, map, 1)?;
    }
    Ok(())
}

fn validate_redirect<S: AsRef<str>>(k: S, v: S, map: &HashMap<String, String>, count: i32) -> Result<()> {

    if count > MAX_REDIRECTS {
        return Err(
            Error::new(
                format!("Too many redirects, limit is {}", MAX_REDIRECTS)));
    }

    if let Some(value) = map.get(v.as_ref()) {
        return validate_redirect(v.as_ref(), value, map, count + 1); 
    }
    Ok(())
}
