use std::path::PathBuf;

use crate::{INDEX_STEM};
use config::{Config, RuntimeOptions, RenderTypes};

use crate::BuildContext;

fn lookup_allow(config: &Config, base: &PathBuf, href: &str) -> Option<PathBuf> {
    if let Some(ref link) = config.link {
        if let Some(ref allow) = link.allow {
            for link in allow {
                let url = link.trim_start_matches("/");
                if url == href {
                    let mut buf = base.clone();
                    buf.push(url);
                    return Some(buf);
                }
            }
        }
    }
    None
}

fn normalize<S: AsRef<str>>(_ctx: &BuildContext, s: S) -> String {
    let mut s = s.as_ref().to_string();
    if !s.starts_with("/") {
        s = format!("/{}", s); 
    }
    // We got a hint with the trailing slash that we should look for an index page
    if s != "/" && s.ends_with("/") {
        s.push_str(config::INDEX_HTML); 
    }
    s
}

// Try to find a source file for the given URL
pub fn lookup(ctx: &BuildContext, href: &str) -> Option<PathBuf> {

    let mut key = normalize(ctx, href);

    //println!("Using the key {:?}", &key);

    if let Some(path) = ctx.collation.links.reverse.get(&key) {
        return Some(path.to_path_buf());
    } else {
        // Sometimes we have directory references without a trailing slash
        // so try again with an index page
        key.push('/');
        key.push_str(config::INDEX_HTML);

        if let Some(path) = ctx.collation.links.reverse.get(&key) {
            return Some(path.to_path_buf());
        //} else {
            //println!("NOT MATCH FOUND FOR {:?}", &key)
        }
    }

    //println!("Searching for link {:?}", href);
    let base = &ctx.options.source;
    // Explicit allow list in site.toml
    if let Some(source) = lookup_allow(&ctx.config, base, href) {
        return Some(source);
    }

    None
}

pub fn exists(ctx: &BuildContext, href: &str) -> bool {
    lookup(ctx, href).is_some()
}

