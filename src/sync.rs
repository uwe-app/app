use crate::{opts::{self, Sync}, Error, Result};

pub async fn run(opts: Sync) -> Result<()> {
    let project = opts::project_path(&opts.project)?;
    if !project.exists() || !project.is_dir() {
        return Err(Error::NotDirectory(project));
    }

    let workspace = workspace::open(&project, true)?;
    for entry in workspace.into_iter() {
        // TODO: use sync branch from config
        println!("Sync command is running ");
    }

    Ok(())
}
