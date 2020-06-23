use crate::cache::{self, CacheComponent};
use crate::preference;
use crate::Result;

#[derive(Debug)]
pub struct UpdateOptions {
    pub blueprint: bool,
    pub standalone: bool,
    pub documentation: bool,
    pub release: bool,
}

pub fn update(options: UpdateOptions) -> Result<()> {
    let prefs = preference::load()?;

    let mut components: Vec<CacheComponent> = vec![
        CacheComponent::Blueprint,
        CacheComponent::Standalone,
        CacheComponent::Documentation,
        CacheComponent::Release,
    ];

    if options.blueprint || options.standalone || options.documentation || options.release {
        components = Vec::new();
        if options.blueprint {
            components.push(CacheComponent::Blueprint);
        }
        if options.standalone {
            components.push(CacheComponent::Standalone);
        }
        if options.documentation {
            components.push(CacheComponent::Documentation);
        }
        if options.release {
            components.push(CacheComponent::Release);
        }
    }

    cache::update(&prefs, components)?;
    Ok(())
}
