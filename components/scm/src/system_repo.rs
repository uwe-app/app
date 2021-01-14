use crate::Result;

pub async fn fetch_releases() -> Result<()> {
    let url = dirs::releases_url();
    let dir = dirs::releases_dir()?;
    crate::clone_or_fetch(&url, &dir)?;
    // Clear the fetch progress
    utils::terminal::clear_current_line()?;
    // Clear the clone messages
    utils::terminal::clear_previous_line()?;
    utils::terminal::clear_previous_line()?;
    Ok(())
}

pub async fn fetch_registry() -> Result<()> {
    let url = dirs::registry_url();
    let dir = dirs::registry_dir()?;
    crate::clone_or_fetch(&url, &dir)?;
    // Clear the fetch progress
    utils::terminal::clear_current_line()?;
    // Clear the clone messages
    utils::terminal::clear_previous_line()?;
    utils::terminal::clear_previous_line()?;
    Ok(())
}
