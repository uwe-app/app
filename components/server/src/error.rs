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

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Warp(#[from] warp::Error),

    #[error(transparent)]
    Notify(#[from] notify::Error),

    #[error(transparent)]
    TrySend(#[from] tokio::sync::mpsc::error::TrySendError<ConnectionInfo>),

    #[error(transparent)]
    SendError(#[from] tokio::sync::mpsc::error::SendError<String>),

    #[error(transparent)]
    Config(#[from] config::Error),

    #[error(transparent)]
    Workspace(#[from] workspace::Error),
}
