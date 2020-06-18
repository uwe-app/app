use std::fs;
use std::path::Path;
use std::path::PathBuf;

use git2::{
    Repository,
    IndexAddOption,
    Cred,
    RemoteCallbacks,
    FetchOptions, 
    ErrorClass,
    Commit,
};
use git2::build::RepoBuilder;

use crate::Error;

// TODO: support [blueprint] default config

pub fn detached<P: AsRef<Path>>(target: P, repo: Repository) -> Result<(), Error> {
    let git_dir = repo.path();

    // Remove the git directory is the easiest
    // way to purge the history
    fs::remove_dir_all(git_dir)?;

    // Create fresh repository
    let new_repo = Repository::init(target)?;

    // Add all the files
    let mut index = new_repo.index()?;
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    // NOTE: must call `write` and `write_tree`
    index.write()?;
    let oid = index.write_tree()?;

    // TODO: get these from preferences when not setand use defaults otherwise
    //let conf = Config::open_default()?;
    //let name = conf.get_string("user.name")?;
    //let email = conf.get_string("user.email")?;

    let sig = repo.signature()?;

    // TODO: allow prefernce for this
    let message = "Initial files.";

    let tree = new_repo.find_tree(oid)?;
    let parents: &[&Commit] = &[];

    let _commit_id = new_repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        parents,
    )?;

    Ok(())
}

pub fn clone_ssh<P: AsRef<Path>>(
    src: String,
    target: P,
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
                return clone_ssh(src, target, key_file.clone(), Some(pass));
            }
        }
    }

    result.map_err(Error::from)
}

