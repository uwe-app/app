use crate::Result;

pub async fn fetch_blueprints() -> Result<()> {
    let url = dirs::blueprints_url();
    let dir = dirs::blueprints_dir()?;
    crate::clone_or_fetch(&url, &dir)?;
    Ok(())
}

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

pub async fn fetch_syntax() -> Result<()> {
    let url = dirs::syntax_url();
    let dir = dirs::syntax_dir()?;
    crate::clone_or_fetch(&url, &dir)?;
    Ok(())
}

pub async fn fetch_documentation() -> Result<()> {
    let url = dirs::documentation_url();
    let dir = dirs::documentation_dir()?;
    crate::clone_or_fetch(&url, &dir)?;
    Ok(())
}
