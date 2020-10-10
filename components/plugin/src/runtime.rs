use crate::Result;

pub async fn fetch() -> Result<()> {
    let url = dirs::get_runtime_url();
    let dir = dirs::get_runtime_dir()?;
    scm::clone_or_fetch(&url, &dir, true)?;
    Ok(())
}
