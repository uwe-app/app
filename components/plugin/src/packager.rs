use std::path::PathBuf;

use futures::TryFutureExt;
use config::Plugin;

use crate::{Result, read, lint, archive::writer::PackageWriter};

/// Package a plugin directory.
pub async fn pack(source: &PathBuf, target: &PathBuf) -> Result<(PathBuf, Vec<u8>, Plugin)> {
    let plugin = read(source).await?;
    lint(&plugin)?;
    pack_plugin(source, target, plugin).await
}

/// Package a plugin.
pub(crate) async fn pack_plugin(source: &PathBuf, target: &PathBuf, plugin: Plugin) -> Result<(PathBuf, Vec<u8>, Plugin)> {
    let writer = PackageWriter::new(source.to_path_buf())
        .destination(target)?
        .tar()
        .and_then(|b| b.xz())
        .and_then(|b| b.digest())
        .await?;
    let (pkg, digest) = writer.into_inner();
    Ok((pkg, digest, plugin))
}
