use std::fs;
use std::path::PathBuf;

use log::info;

use crate::{
    opts::{self, Task},
    Error, Result,
};
use config::plugin::dependency::DependencyTarget;
use plugin::new_registry;

//use super::alias;

const CYPRESS_JSON: &str = "cypress.json";
const CYPRESS_OPTS: &str = "cypress.opts";
const OPEN_SPEC: &str = "open.spec.js";
const DOWNLOADS: &str = "downloads";
const FIXTURES: &str = "fixtures";
const INTEGRATION: &str = "integration";
const SCREENSHOTS: &str = "screenshots";
const VIDEOS: &str = "videos";

pub async fn run(cmd: Task) -> Result<()> {
    match cmd {
        Task::ListBlueprints {} => {
            list_blueprints().await?;
        }
        Task::InitTest {
            project,
            folder_name,
        } => {
            let project = opts::project_path(&project)?;
            init_test(project, folder_name).await?;
        }
        Task::CheckDeps { project } => {
            let project = opts::project_path(&project)?;
            check_deps(project).await?;
        }
        //Task::Alias { cmd } => {
            //alias::run(cmd).await?;
        //}
    }
    Ok(())
}

/// List standard blueprints.
async fn list_blueprints() -> Result<()> {
    let namespace = config::PLUGIN_BLUEPRINT_NAMESPACE;
    let registry = new_registry()?;
    let entries = registry.starts_with(namespace).await?;
    for (name, entry) in entries.iter() {
        let (version, item) = entry.latest().unwrap();
        let short_name = item.short_name().unwrap();
        info!("{} ({}@{})", short_name, name, version.to_string());
    }
    Ok(())
}

/// Create the integration test folder structure.
async fn init_test(project: PathBuf, name: String) -> Result<()> {
    let test_name = PathBuf::from(&name);
    if test_name.is_absolute() {
        return Err(Error::NotRelative(test_name));
    }

    let cypress_opts = r#"
--config-file
test/cypress.json
--reporter-options
--no-color
"#;

    let cypress_content = format!(
        r#"{{
  "downloadsFolder": "{}",
  "fixturesFolder": "{}",
  "integrationFolder": "{}",
  "screenshotsFolder": "{}",
  "videosFolder": "{}",
  "pluginsFile": false,
  "supportFile": false
}}"#,
        format!("{}/{}", &name, DOWNLOADS),
        format!("{}/{}", &name, FIXTURES),
        format!("{}/{}", &name, INTEGRATION),
        format!("{}/{}", &name, SCREENSHOTS),
        format!("{}/{}", &name, VIDEOS),
    );

    let spec_content = r#"
describe('Open the site', () => {
  it('Visits the index page', () => {
    cy.visit('/');
  })
})
"#;

    let workspace = workspace::open(&project, true, &vec![])?;
    for config in workspace.into_iter() {
        let cypress_json = config.project().join(&name).join(CYPRESS_JSON);

        if cypress_json.exists() {
            return Err(Error::NoOverwriteTestSpec(cypress_json.to_path_buf()));
        }

        info!("Init test {}", config.project().display());

        let dirs = vec![
            config.project().join(&name).join(DOWNLOADS),
            config.project().join(&name).join(FIXTURES),
            config.project().join(&name).join(INTEGRATION),
            config.project().join(&name).join(SCREENSHOTS),
            config.project().join(&name).join(VIDEOS),
        ];

        // Create the directories
        for d in dirs {
            fs::create_dir_all(&d)?;
            info!("Created {} ✓", d.display());
        }

        // Write the configuration settings
        fs::write(&cypress_json, &cypress_content)?;
        info!("Created {} ✓", cypress_json.display());

        // Create a stub test spec if possible
        let open_spec = config
            .project()
            .join(&name)
            .join(INTEGRATION)
            .join(OPEN_SPEC);
        if !open_spec.exists() {
            fs::write(&open_spec, &spec_content)?;
            info!("Created {} ✓", open_spec.display());
        }

        let opts = config.project().join(&name).join(CYPRESS_OPTS);
        if !opts.exists() {
            fs::write(&opts, &cypress_opts)?;
            info!("Created {} ✓", opts.display());
        }

        info!("Done ✓");
    }

    Ok(())
}

/// Check plugin dependencies do not use `path` or `archive`
/// local references.
async fn check_deps(project: PathBuf) -> Result<()> {
    let workspace = workspace::open(&project, true, &vec![])?;
    for config in workspace.into_iter() {
        if let Some(ref deps) = config.dependencies() {
            for (name, dep) in deps.iter() {
                if let Some(ref target) = dep.target {
                    match target {
                        DependencyTarget::File { path } => {
                            return Err(Error::LocalDependencyNotAllowed(
                                path.to_path_buf(),
                            ))
                        }
                        DependencyTarget::Archive { archive } => {
                            return Err(Error::LocalDependencyNotAllowed(
                                archive.to_path_buf(),
                            ))
                        }
                        _ => {}
                    }
                }
                info!("Dependency {} is ok ✓", name)
            }
        }
    }

    Ok(())
}
