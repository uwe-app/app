use crate::Result;

pub async fn fetch() -> Result<()> {
    let url = dirs::runtime_url();
    let dir = dirs::runtime_dir()?;
    scm::clone_or_fetch(&url, &dir)?;
    Ok(())
}

pub async fn fetch_releases() -> Result<()> {
    let url = dirs::releases_url();
    let dir = dirs::releases_dir()?;
    scm::clone_or_fetch(&url, &dir)?;
    Ok(())
}

pub async fn fetch_registry() -> Result<()> {
    let url = dirs::registry_url();
    let dir = dirs::registry_dir()?;
    scm::clone_or_fetch(&url, &dir)?;
    Ok(())
}

pub async fn fetch_syntax() -> Result<()> {
    let url = dirs::syntax_url();
    let dir = dirs::syntax_dir()?;
    scm::clone_or_fetch(&url, &dir)?;
    Ok(())
}
