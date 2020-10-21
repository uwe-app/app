use crate::Result;

// Fetch the updated runtime repository.
pub async fn fetch() -> Result<()> {
    let url = dirs::runtime_url();
    let dir = dirs::runtime_dir()?;
    scm::clone_or_fetch(&url, &dir)?;
    Ok(())
}
