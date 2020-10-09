use cache::CacheComponent;

use crate::Result;

pub async fn fetch() -> Result<()> {
    let components = vec![CacheComponent::Runtime];
    cache::update(components)?;
    Ok(())
}
