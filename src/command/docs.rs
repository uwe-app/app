use crate::cache::{self, CacheComponent};
use crate::preference;
use crate::{utils, Error};

use super::run::{self, ServeOptions};

static DOCS_DIR: &str = "docs";

#[derive(Debug)]
pub struct DocsOptions {}

pub fn docs(_: DocsOptions) -> Result<(), Error> {
    let prefs = preference::load()?;
    let docs_prefs = prefs.docs.as_ref().unwrap();

    // Served from a sub-directory
    let mut target = cache::get_docs_dir()?;

    if !target.exists() {
        cache::update(&prefs, vec![CacheComponent::Documentation])?;
    }

    target.push(DOCS_DIR);

    let opts = ServeOptions {
        target,
        host: docs_prefs.host.clone(),
        port: docs_prefs.port.clone(),
        open_browser: true,
        watch: None,
        endpoint: utils::generate_id(16),
        redirects: None,
    };

    run::serve_only(opts)
}
