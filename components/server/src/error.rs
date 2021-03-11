use std::path::PathBuf;

use config::server::ConnectionInfo;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to get user name when dropping privileges (getlogin)")]
    DropPrivilegeGetLogin,

    #[error("Failed to get user info when dropping privileges (getpwnam)")]
    DropPrivilegeGetInfo,

    #[error("Failed to drop privileges")]
    DropPrivilegeFail,

    #[error("Failed to set group when dropping privileges (setgid)")]
    DropPrivilegeGroup,

    #[error("Failed to set user when dropping privileges (setuid)")]
    DropPrivilegeUser,

    #[error("Failed to get user id when dropping privileges (getuid)")]
    DropPrivilegeGetUserId,

    #[error("No virtual hosts for live reload")]
    NoLiveHosts,

    #[error("Unable to open SSL certificate file {0}")]
    SslCertFile(PathBuf),

    #[error("Unable to open SSL key file {0}")]
    SslKeyFile(PathBuf),

    #[error("Unable to construct SSL certificate chain from {0}")]
    SslCertChain(PathBuf),

    #[error("Unable to load SSL private key from {0}")]
    SslPrivateKey(PathBuf),

    #[error("No PKCS8 keys decoded from SSL key file {0}")]
    SslKeyRead(PathBuf),

    #[error("Web server requires some virtual hosts")]
    NoVirtualHosts,

    #[error("Virtual host requires a name")]
    NoVirtualHostName,

    #[error("Virtual host '{0}' requires a directory")]
    NoVirtualHostDirectory(String),

    #[error("The virtual host '{0}' has a directory '{1}' which does not exist or is not a directory")]
    VirtualHostDirectory(String, PathBuf),

    #[error("The virtual host {0} expects the index file {1}")]
    NoIndexFile(String, PathBuf),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Notify(#[from] notify::Error),

    #[error(transparent)]
    Tls(#[from] rustls::TLSError),

    #[error(transparent)]
    TrySend(#[from] tokio::sync::mpsc::error::TrySendError<ConnectionInfo>),

    #[error(transparent)]
    SendError(#[from] tokio::sync::mpsc::error::SendError<String>),

    #[error(transparent)]
    Psup(#[from] psup_impl::Error),

    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Workspace(#[from] workspace::Error),
}
