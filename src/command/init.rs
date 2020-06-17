use std::fs;
use std::path::Path;
use std::path::PathBuf;

use home;
use url::Url;
use git2::Repository;
use log::info;

use crate::blueprint;
use crate::Error;

#[derive(Debug)]
pub struct InitOptions {
    pub source: String,
    pub target: Option<PathBuf>,
    pub list: bool,
    pub fetch: bool,
    pub private_key: Option<PathBuf>,
}

fn clone_ssh<P: AsRef<Path>>(
    target: P,
    options: &InitOptions,
    key_file: PathBuf,
    password: Option<String>) -> Result<Repository, Error> {

    let passphrase = if let Some(ref phrase) = password {
        Some(phrase.as_str()) 
    } else {
        None
    };

    let private_key = key_file.as_path();

    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        git2::Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            private_key,
            passphrase,
        )
    });

    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    let result = builder.clone(
        &options.source,
        target.as_ref(),
    );

    if let Err(ref e) = result {
        if let git2::ErrorClass::Ssh = e.class() {
            // Sadly cannot find a better way to detect this particular error
            if e.message().contains("Wrong passphrase") {
                let pass = rpassword::read_password_from_tty(Some("Passphrase: "))?;
                return clone_ssh(target, options, key_file.clone(), Some(pass));
            }
        }
    }

    result.map_err(Error::from)
}

fn create<P: AsRef<Path>>(target: P, options: &InitOptions) -> Result<Repository, Error> {
    println!("{:?}", options);

    let (repo, base, _cloned) = blueprint::open_or_clone()?;
    match Url::parse(&options.source) {
        Ok(_) => {
            info!("Clone {}", &options.source);
            info!("   -> {}", target.as_ref().display());
            return Repository::clone(&options.source, target)
                .map_err(Error::from);
        },
        Err(_) => {
            let modules = repo.submodules()?;
            for sub in modules {
                if sub.path() == Path::new(&options.source) {
                    let mut tmp = base.clone();
                    tmp.push(sub.path());
                    let src = tmp.to_string_lossy();
                    info!("Clone {}", tmp.display());
                    info!("   -> {}", target.as_ref().display());
                    return Repository::clone(&src, target)
                        .map_err(Error::from);
                }
            }

            // Now we have SSH style git@github.com: URLs to deal with
            
            if let Some(mut key_file) = home::home_dir() {
                if let Some(ref ssh_key) = options.private_key {
                    key_file.push(ssh_key);

                    info!("Private key {}", key_file.display());
                    info!("Clone {}", &options.source);
                    info!("   -> {}", target.as_ref().display());

                    return clone_ssh(target, options, key_file, None);
                } else {
                    return Err(
                        Error::new(
                            format!("To use SSH specify the --ssh-key option")))
                }
            }

        }
    }

    Err(Error::new(format!("Unable to handle source specification")))
}

// TODO: support [blueprint] default config

pub fn init(options: InitOptions) -> Result<(), Error> {

    if options.list {
        let (repo, _base, _cloned) = blueprint::open_or_clone()?;
        blueprint::list_submodules(repo)?;
    } else if options.fetch {
        blueprint::clone_or_fetch()?;
    } else {

        if let Some(ref target) = options.target {
            if target.exists() {
                return Err(
                    Error::new(
                        format!("Target '{}' exists, please move it away", target.display())));
            }

            if let Some(ref parent) = target.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
                create(target, &options)?;
            } else {
                create(target, &options)?;
            }
        } else {
            return Err(Error::new(format!("Target directory is required")));
        }
    }

    Ok(())
}
