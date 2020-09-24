use cache::{self, CacheComponent};
use preference;

use crate::Result;

#[derive(Debug)]
pub struct FetchOptions {
    pub blueprint: bool,
    pub release: bool,
    pub syntax: bool,
}

pub fn update(options: FetchOptions) -> Result<()> {
    let prefs = preference::load()?;

    let mut components: Vec<CacheComponent> = vec![
        CacheComponent::Blueprint,
        CacheComponent::Release,
        CacheComponent::Syntax,
    ];

    if options.blueprint
        || options.release
        || options.syntax
    {
        components = Vec::new();

        if options.blueprint {
            components.push(CacheComponent::Blueprint);
        }
        if options.release {
            components.push(CacheComponent::Release);
        }
        if options.syntax {
            components.push(CacheComponent::Syntax);
        }
    }

    cache::update(&prefs, components)?;
    Ok(())
}
