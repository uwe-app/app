use git2::{Cred, RemoteCallbacks};

pub fn ssh_agent<'a>() -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        let username = username_from_url
            .or(option_env!("USER"))
            .unwrap_or("nobody");
        Cred::ssh_key_from_agent(username)
    });
    callbacks
}
