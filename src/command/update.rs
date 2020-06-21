use crate::preference;
use crate::cache::{self, CacheComponent};
use crate::Error;

#[derive(Debug)]
pub struct UpdateOptions {
    pub blueprint: bool,
    pub standalone: bool,
    pub documentation: bool,
}

pub fn update(options: UpdateOptions) -> Result<(), Error> {
    let prefs = preference::load()?;

    let mut components: Vec<CacheComponent> = vec![
        CacheComponent::Blueprint,
        CacheComponent::Standalone,
        CacheComponent::Documentation,
    ];

    if options.blueprint || options.standalone || options.documentation {
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
    }

    cache::update(&prefs, components)?;
    Ok(())
}
