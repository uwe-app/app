use cache::{self, CacheComponent};

use crate::Result;

#[derive(Debug)]
pub struct FetchOptions {
    pub release: bool,
}

pub fn update(options: FetchOptions) -> Result<()> {
    let mut components: Vec<CacheComponent> =
        vec![CacheComponent::Release];

    if options.release {
        components = Vec::new();
        if options.release {
            components.push(CacheComponent::Release);
        }
    }

    cache::update(components)?;
    Ok(())
}
