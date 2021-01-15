use std::path::PathBuf;

use crate::Result;

pub async fn fetch_releases() -> Result<()> {
    let url = dirs::releases_url();
    let dir = dirs::releases_dir()?;
    fetch(url, dir).await
}

pub async fn fetch_registry() -> Result<()> {
    let url = dirs::registry_url();
    let dir = dirs::registry_dir()?;
    fetch(url, dir).await
}

async fn fetch(url: String, dir: PathBuf) -> Result<()> {
    let (_, cloned) = crate::clone_or_fetch(&url, &dir)?;

    // Clear the fetch progress
    utils::terminal::clear_current_line()?;

    // Clear the main message
    utils::terminal::clear_previous_line()?;

    // Cloning prints two lines
    if cloned {
        utils::terminal::clear_previous_line()?;
    }
    Ok(())
}
