use cache::CacheComponent;

use crate::Result;

pub async fn update() -> Result<()> {
    let components = vec![CacheComponent::Runtime];
    cache::update(components)?;
    Ok(())
}
