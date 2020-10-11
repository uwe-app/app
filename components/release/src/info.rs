use log::info;
use semver::Version;

use crate::Result;

pub(crate) fn upto_date(version: &Version) -> Result<()> {
    info!("Version {} is up to date ✓", version.to_string());
    return Ok(());
}
