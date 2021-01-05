use crate::Result;

// Fetch the updated runtime repository.
pub async fn fetch_registry() -> Result<()> {
    let url = dirs::registry_url();
    let dir = dirs::registry_dir()?;
    scm::clone_or_fetch(&url, &dir)?;
    Ok(())
}
