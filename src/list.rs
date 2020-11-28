use log::info;

use crate::Result;

pub async fn list_blueprints() -> Result<()> {
    let blueprints = dirs::blueprint_dir()?;
    for entry in std::fs::read_dir(blueprints)? {
        let path = entry?.path();
        if path.is_dir() {
            let name = path.file_name().unwrap().to_string_lossy();
            info!("{} ({})", &*name, path.display());
        }
    }
    Ok(())
}
