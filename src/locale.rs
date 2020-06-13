use std::path::Path;

use fluent_templates::ArcLoader;
use unic_langid::LanguageIdentifier;

pub fn loader<P: AsRef<Path>>(dir: P, fallback: LanguageIdentifier) -> Result<ArcLoader, Box<dyn std::error::Error>> {
    let file = dir.as_ref();
    let mut core = file.to_path_buf();
    core.push("core.ftl");
    let builder = ArcLoader::builder(dir.as_ref(), fallback)
        .shared_resources(Some(&[core])).build();
        //.customize(|bundle| bundle.set_use_isolating(false));

    builder
}
