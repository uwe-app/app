use log::info;

use crate::{
    opts::{self, Test},
    Error, Result,
};

pub async fn run(opts: Test) -> Result<()> {
    let project = opts::project_path(&opts.project)?;

    let workspace = workspace::open(&project, true, &vec![])?;
    for config in workspace.into_iter() {
        println!("Running tests {:?}", config.project());
    }

    Ok(())
}
