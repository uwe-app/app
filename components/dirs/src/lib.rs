use std::path::PathBuf;
use std::{fs, io};

const ROOT_DIR: &str = ".uwe";

const BIN: &str = "bin";
const ENV: &str = "env";

const SITES_NAME: &str = "sites";
const SITES_FILE: &str = "sites.toml";

const RELEASES_REPO: &str = "https://github.com/uwe-app/releases";
const REGISTRY_REPO: &str = "https://github.com/uwe-app/registry";

/// Name of the releases reppsitory.
const RELEASES: &str = "releases";

/// Name of the plugin registry repository.
const REGISTRY: &str = "registry";
/// Name for the location of cached plugin downloads.
const DOWNLOADS: &str = "downloads";
/// Name for the location of registry packages (JSON files).
const PACKAGES: &str = "packages";
/// Name for the location of cached plugin repositories.
const REPOSITORIES: &str = "repositories";
/// Name for the location of where plugins installed from archives are placed.
const ARCHIVES: &str = "archives";

/// Get the root directory (~/.uwe) but do not
/// create it if it does not exist.
pub fn root_dir() -> io::Result<PathBuf> {
    home::home_dir().map(|p| p.join(ROOT_DIR)).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            String::from("Could not determine home directory"),
        )
    })
}

pub fn sites_dir() -> io::Result<PathBuf> {
    let mut bin = root_dir()?;
    bin.push(SITES_NAME);
    if !bin.exists() {
        fs::create_dir(&bin)?;
    }
    Ok(bin)
}

pub fn sites_manifest() -> io::Result<PathBuf> {
    Ok(root_dir()?.join(SITES_FILE))
}

pub fn env_file() -> io::Result<PathBuf> {
    let mut env = root_dir()?;
    env.push(ENV);
    Ok(env)
}

pub fn bin_dir() -> io::Result<PathBuf> {
    let mut bin = root_dir()?;
    bin.push(BIN);
    if !bin.exists() {
        fs::create_dir(&bin)?;
    }
    Ok(bin)
}

pub fn releases_url() -> String {
    RELEASES_REPO.to_string()
}

pub fn registry_url() -> String {
    REGISTRY_REPO.to_string()
}

pub fn releases_dir() -> io::Result<PathBuf> {
    Ok(root_dir()?.join(RELEASES))
}

pub fn registry_dir() -> io::Result<PathBuf> {
    Ok(root_dir()?.join(REGISTRY))
}

pub fn packages_dir() -> io::Result<PathBuf> {
    Ok(registry_dir()?.join(PACKAGES))
}

pub fn downloads_dir() -> io::Result<PathBuf> {
    Ok(registry_dir()?.join(DOWNLOADS))
}

pub fn archives_dir() -> io::Result<PathBuf> {
    Ok(registry_dir()?.join(ARCHIVES))
}

pub fn repositories_dir() -> io::Result<PathBuf> {
    Ok(registry_dir()?.join(REPOSITORIES))
}

pub use home::home_dir;
