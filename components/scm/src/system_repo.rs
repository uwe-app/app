use crate::Result;

pub async fn fetch_releases() -> Result<()> {
    let url = dirs::releases_url();
    let dir = dirs::releases_dir()?;
    crate::clone_or_fetch(&url, &dir)?;
    Ok(())
}

pub async fn fetch_registry() -> Result<()> {
    let url = dirs::registry_url();
    let dir = dirs::registry_dir()?;
    crate::clone_or_fetch(&url, &dir)?;
    Ok(())
}
