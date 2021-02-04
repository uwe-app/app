use crate::{
    opts::{self, Test},
    Result,
};

use config::{ProfileName, ProfileSettings};

use workspace::{default_compiler, build, ProjectBuilder, Project};

pub async fn run(opts: Test) -> Result<()> {
    let project = opts::project_path(&opts.project)?;
    let args = ProfileSettings::from(&ProfileName::Release);
    build(&project, &args, test_compiler).await?;
    Ok(())
}

async fn test_compiler(builder: ProjectBuilder) -> workspace::Result<Project> {
    let project = default_compiler(builder).await?;

    //let (shutdown_tx, shutdown_rx) = futures::channel::oneshot::channel::<()>();

    println!("Project was compiled for test runner...");

    // TODO: launch test server
    // TODO: run cypress

    Ok(project)
}
