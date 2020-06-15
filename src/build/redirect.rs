use std::fs;
use log::info;

use super::context::Context;
use crate::{utils, Error, INDEX_HTML};
use crate::content::redirect;

pub fn write(context: &Context) -> Result<(), Error> {
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
                return Err(
                    Error::new(
                        format!("Redirect file '{}' exists", buf.display())));
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
