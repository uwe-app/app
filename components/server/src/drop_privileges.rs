// Copyright (c) 2019 Cloudflare, Inc. All rights reserved.
// SPDX-License-Identifier: BSD-3-Clause
//
// Derived from: https://github.com/cloudflare/boringtun/blob/master/src/device/drop_privileges.rs

use libc::*;
use log::{debug, info};

use crate::{Error, Result};

fn get_saved_ids() -> Result<(uid_t, gid_t, String)> {
    // Get the user name of the sudoer
    let uname = unsafe { getlogin() };
    if uname.is_null() {
        return Err(Error::DropPrivilegeGetLogin);
    }
    let userinfo = unsafe { getpwnam(uname) };
    if userinfo.is_null() {
        return Err(Error::DropPrivilegeGetInfo);
    }

    // Saved group ID
    let saved_gid = unsafe { (*userinfo).pw_gid };
    // Saved user ID
    let saved_uid = unsafe { (*userinfo).pw_uid };

    let username = unsafe { std::ffi::CStr::from_ptr((*userinfo).pw_name) };
    let username = username.to_string_lossy().into_owned();

    Ok((saved_uid, saved_gid, username))
}

pub(crate) fn is_root() -> bool {
    0 == unsafe { getuid() }
}

pub(crate) fn drop_privileges() -> Result<()> {

    debug!("Dropping privileges...");

    let (saved_uid, saved_gid, username) = get_saved_ids()?;

    if -1 == unsafe { setgid(saved_gid) } {
        // Set real and effective group ID
        return Err(Error::DropPrivilegeGroup);
    }

    if -1 == unsafe { setuid(saved_uid) } {
        // Set  real and effective user ID
        return Err(Error::DropPrivilegeUser);
    }

    // Validated we can't get sudo back again
    if unsafe { (setgid(0) != -1) || (setuid(0) != -1) } {
        Err(Error::DropPrivilegeFail)
    } else {
        info!("Dropped privileges to {}", username);
        Ok(())
    }
}
