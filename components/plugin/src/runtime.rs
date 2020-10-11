use crate::Result;

pub async fn fetch() -> Result<()> {
    // FIXME: call out to `uvm runtime`
    let url = dirs::runtime_url();
    let dir = dirs::runtime_dir()?;
    scm::clone_or_fetch(&url, &dir)?;
    Ok(())
}
