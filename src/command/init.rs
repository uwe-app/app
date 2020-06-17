use std::fs;
use std::path::Path;
use std::path::PathBuf;

use home;
use url::Url;
use git2::{
    Repository,
    IndexAddOption,
    Cred,
    RemoteCallbacks,
    FetchOptions, 
    ErrorClass,
    Config,
    Signature,
    Commit,
};
use git2::build::RepoBuilder;
use log::info;

use crate::blueprint;
use crate::preference::{self, Preferences};
use crate::Error;

// TODO: support [blueprint] default config

#[derive(Debug)]
pub struct InitOptions {
    pub source: Option<String>,
    pub target: Option<PathBuf>,
    pub list: bool,
    pub fetch: bool,
    pub private_key: Option<PathBuf>,
}

fn fresh<P: AsRef<Path>>(target: P, repo: Repository) -> Result<(), Error> {
    let git_dir = repo.path();

    println!("Fresh {}", git_dir.display());

    // Remove the git directory is the easiest
    // way to purge the history
    fs::remove_dir_all(git_dir)?;

    // Create fresh repository
    let new_repo = Repository::init(target)?;

    println!("After init new repo {}", new_repo.path().display());

    // Add all the files
    let mut index = new_repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    let oid = index.write_tree()?;

    println!("After adding files to the index");

    let conf = Config::open_default()?;

    println!("Got config");

    // TODO: get these from preferences when not setand use defaults otherwise
    let name = conf.get_string("user.name")?;
    let email = conf.get_string("user.email")?;

    println!("Got name {}", &name);
    println!("Got email {}", &email);

    let author = Signature::now(&name, &email)?;
    let committer = Signature::now(&name, &email)?;

    // TODO: allow prefernce for this
    let message = "Initial files.";

    //let head = new_repo.head()?;

    println!("Got head reference");

    //if let Some(oid) = head.target() {

        println!("Got oid {:?}", oid);

        let tree = new_repo.find_tree(oid)?;

        println!("Found the tree {:?}", tree);

        let parents: &[&Commit] = &[];

        println!("Making the commit to the repo");

        let commit_id = new_repo.commit(
            None,
            &author,
            &committer,
            message,
            &tree,
            parents,
        )?;

        println!("Got commit it {:?}", commit_id);
    //}


    Ok(())
}

fn clone_ssh<P: AsRef<Path>>(
    src: String,
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

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            private_key,
            passphrase,
        )
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fo);

    let result = builder.clone(
        &src,
        target.as_ref(),
    );

    if let Err(ref e) = result {
        if let ErrorClass::Ssh = e.class() {
            // Sadly cannot find a better way to detect this particular error
            if e.message().contains("Wrong passphrase") {
                let pass = rpassword::read_password_from_tty(Some("Passphrase: "))?;
                return clone_ssh(src, target, options, key_file.clone(), Some(pass));
            }
        }
    }

    result.map_err(Error::from)
}

fn create<P: AsRef<Path>>(target: P, options: &InitOptions, prefs: &Preferences) -> Result<Repository, Error> {
    let mut src = "".to_string();

    if let Some(ref source) = options.source {
        src = source.clone();
    } else {
        if let Some(ref source) = prefs.blueprint.as_ref().unwrap().default_path {
            src = source.clone();
        }
    }

    if src.is_empty() {
        return Err(
            Error::new(
                format!("Could not determine default source path")));
    }

    let src_err = Err(
        Error::new(
            format!("Unable to handle source '{}'", &src)));

    let (repo, base, _cloned) = blueprint::open_or_clone()?;
    match Url::parse(&src) {
        Ok(_) => {
            info!("Clone {}", &src);
            info!("   -> {}", target.as_ref().display());
            return Repository::clone(&src, target)
                .map_err(Error::from);
        },
        Err(_) => {
            let modules = repo.submodules()?;
            for sub in modules {
                if sub.path() == Path::new(&src) {
                    let mut tmp = base.clone();
                    tmp.push(sub.path());
                    let src = tmp.to_string_lossy();
                    info!("Clone {}", tmp.display());
                    info!("   -> {}", target.as_ref().display());
                    return Repository::clone(&src, target)
                        .map_err(Error::from);
                }
            }

            let ssh_req = src.trim_start_matches("ssh://");
            let ssh_url = format!("ssh://{}", &ssh_req);

            match Url::parse(&ssh_url) {
                Ok(url) => {
                    if url.username().is_empty() {
                        log::warn!("No username for source URL");
                        log::warn!("Perhaps you want a blueprint submodule, try `ht init --list`");
                        return src_err;
                    }

                    // Now we have SSH style git@github.com: URLs to deal with
                    if let Some(mut key_file) = home::home_dir() {
                        if let Some(ref ssh_key) = options.private_key {
                            key_file.push(ssh_key);

                            info!("Private key {}", key_file.display());
                            info!("Clone {}", &src);
                            info!("   -> {}", target.as_ref().display());

                            return clone_ssh(src, target, options, key_file, None);
                        } else {
                            return Err(
                                Error::new(
                                    format!("To use SSH specify the --ssh-key option")))
                        }
                    }
                },
                Err(_) => {
                    return src_err
                },
            }

        }
    }

    src_err
}

pub fn init(options: InitOptions) -> Result<(), Error> {
    let prefs = preference::load()?;

    let (will_clone, dest, url) = blueprint::will_clone()?;
    if will_clone {
        info!("Clone {} -> {}", url, dest.display());
    }

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

            let repo;
            let fresh_target = target.clone();

            if let Some(ref parent) = target.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
                repo = create(target, &options, &prefs)?;
            } else {
                repo = create(target, &options, &prefs)?;
            }

            //repo.remote_delete("origin")?;

            fresh(&fresh_target, repo)?;
        } else {
            return Err(Error::new(format!("Target directory is required")));
        }
    }

    Ok(())
}
