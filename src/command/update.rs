use crate::preference;
use crate::cache::{self, CacheComponent};
use crate::Error;

#[derive(Debug)]
pub struct UpdateOptions {
    pub blueprint: bool,
    pub standalone: bool,
}

pub fn update(options: UpdateOptions) -> Result<(), Error> {
    let prefs = preference::load()?;

    let mut components: Vec<CacheComponent> = vec![
        CacheComponent::Blueprint,
        CacheComponent::Standalone,
    ];

    if options.blueprint || options.standalone {
        components = Vec::new();
        if options.blueprint {
            components.push(CacheComponent::Blueprint);
        }
        if options.standalone{
            components.push(CacheComponent::Standalone);
        }
    }

    cache::update(&prefs, components)?;
    Ok(())
}
