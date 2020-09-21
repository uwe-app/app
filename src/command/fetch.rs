use cache::{self, CacheComponent};
use preference;

use crate::Result;

#[derive(Debug)]
pub struct FetchOptions {
    pub blueprint: bool,
    pub documentation: bool,
    pub release: bool,
    pub syntax: bool,
    pub book: bool,
}

pub fn update(options: FetchOptions) -> Result<()> {
    let prefs = preference::load()?;

    let mut components: Vec<CacheComponent> = vec![
        CacheComponent::Blueprint,
        CacheComponent::Documentation,
        CacheComponent::Release,
        CacheComponent::Syntax,
        CacheComponent::Book,
    ];

    if options.blueprint
        || options.documentation
        || options.release
        || options.syntax
        || options.book
    {
        components = Vec::new();

        if options.blueprint {
            components.push(CacheComponent::Blueprint);
        }
        if options.documentation {
            components.push(CacheComponent::Documentation);
        }
        if options.release {
            components.push(CacheComponent::Release);
        }
        if options.syntax {
            components.push(CacheComponent::Syntax);
        }
        if options.book {
            components.push(CacheComponent::Book);
        }
    }

    cache::update(&prefs, components)?;
    Ok(())
}
