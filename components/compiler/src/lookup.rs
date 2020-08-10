use crate::BuildContext;
use std::path::PathBuf;

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
    if let Some(path) = ctx.collation.links.reverse.get(&key) {
        return Some(path.to_path_buf());
    } else {
        // Sometimes we have directory references without a trailing slash
        // so try again with an index page
        key.push('/');
        key.push_str(config::INDEX_HTML);
        if let Some(path) = ctx.collation.links.reverse.get(&key) {
            return Some(path.to_path_buf());
        }
    }

    None
}

pub fn exists(ctx: &BuildContext, href: &str) -> bool {
    lookup(ctx, href).is_some()
}
