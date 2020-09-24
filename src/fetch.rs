use cache::{self, CacheComponent};
use preference;

use crate::Result;

#[derive(Debug)]
pub struct FetchOptions {
    pub blueprint: bool,
    pub release: bool,
}

pub fn update(options: FetchOptions) -> Result<()> {
    let prefs = preference::load()?;

    let mut components: Vec<CacheComponent> = vec![
        CacheComponent::Blueprint,
        CacheComponent::Release,
    ];

    if options.blueprint
        || options.release
    {
        components = Vec::new();

        if options.blueprint {
            components.push(CacheComponent::Blueprint);
        }
        if options.release {
            components.push(CacheComponent::Release);
        }
    }

    cache::update(&prefs, components)?;
    Ok(())
}
