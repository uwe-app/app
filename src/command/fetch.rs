use cache::{self, CacheComponent};
use preference;

use crate::Result;

#[derive(Debug)]
pub struct FetchOptions {
    pub blueprint: bool,
    pub standalone: bool,
    pub documentation: bool,
    pub release: bool,
    pub short_code: bool,
    pub syntax: bool,
    pub search: bool,
    pub feed: bool,
    pub book: bool,
}

pub fn update(options: FetchOptions) -> Result<()> {
    let prefs = preference::load()?;

    let mut components: Vec<CacheComponent> = vec![
        CacheComponent::Blueprint,
        CacheComponent::Standalone,
        CacheComponent::Documentation,
        CacheComponent::Release,
        CacheComponent::ShortCode,
        CacheComponent::Syntax,
        CacheComponent::Search,
        CacheComponent::Feed,
        CacheComponent::Book,
    ];

    if options.blueprint
        || options.standalone
        || options.documentation
        || options.release
        || options.short_code
        || options.syntax
        || options.search
        || options.feed
        || options.book
    {
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
        if options.short_code {
            components.push(CacheComponent::ShortCode);
        }
        if options.syntax {
            components.push(CacheComponent::Syntax);
        }
        if options.search {
            components.push(CacheComponent::Search);
        }
        if options.feed {
            components.push(CacheComponent::Feed);
        }
        if options.book {
            components.push(CacheComponent::Book);
        }
    }

    cache::update(&prefs, components)?;
    Ok(())
}
