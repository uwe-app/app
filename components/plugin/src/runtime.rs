use crate::{Error, Result};

// Fetch the updated runtime repository.
pub async fn fetch() -> Result<()> {
    utils::command::run("uvm", &["runtime"], None)
        .map_err(Error::from)
}
