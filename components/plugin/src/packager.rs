use std::path::PathBuf;

use config::Plugin;
use futures::TryFutureExt;

use crate::{
    archive::writer::PackageWriter, linter::lint_plugin, reader::read, Result,
};

/// Package a plugin directory.
pub async fn pack(
    source: &PathBuf,
    target: &PathBuf,
) -> Result<(PathBuf, Vec<u8>, Plugin)> {
    let plugin = read(source).await?;
    lint_plugin(&plugin)?;
    pack_plugin(source, target, plugin).await
}

/// Package a plugin.
pub(crate) async fn pack_plugin(
    source: &PathBuf,
    target: &PathBuf,
    plugin: Plugin,
) -> Result<(PathBuf, Vec<u8>, Plugin)> {
    let writer = PackageWriter::new(source.to_path_buf())
        .destination(target)?
        .tar()
        .and_then(|b| b.xz())
        .and_then(|b| b.digest())
        .await?;
    let (pkg, digest) = writer.into_inner();
    Ok((pkg, digest, plugin))
}
