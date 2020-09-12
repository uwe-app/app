use std::path::PathBuf;

use crate::BuildContext;
use collator::LinkCollate;

// Try to find a source file for the given URL
pub fn lookup(ctx: &BuildContext, href: &str) -> Option<PathBuf> {
    let collation = &*ctx.collation.read().unwrap();
    let mut key = collation.normalize(href);
    if let Some(path) = collation.get_link(&key) {
        return Some(path.to_path_buf());
    } else {
        // Sometimes we have directory references without a trailing slash
        // so try again with an index page
        key.push('/');
        key.push_str(config::INDEX_HTML);
        if let Some(path) = collation.get_link(&key) {
            return Some(path.to_path_buf());
        }
    }
    None
}

pub fn exists(ctx: &BuildContext, href: &str) -> bool {
    lookup(ctx, href).is_some()
}
