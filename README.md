# Universal Web Editor

Repositories must be siblings, for example:

```
.
├── app
├── community
├── integrations
├── library
├── plugins
├── releases
├── runtime
├── sites
└── syntax
```

* `app`: Main source code (this repository).
* `blog`: Canonical blog website.
* `library`: Additional libraries; search runtime and third-party forks.
* `plugins`: Source code for the plugins.
* `runtime`: Runtime library; syntax highlighting and registry cache etc.
* `syntax`: Builds the syntax highlighting runtime files to the `runtime`.
* `website`: Source code for the project website.

## Releases

A private executable `release-manager` performs all the steps for a release.

1) Bump the version in `Cargo.toml`.
2) Publish a new release: `cargo run --bin=release-manager`.
3) Commit and push the new release version and checksums in the [runtime][] repository (`releases.json`).

If you need them `release-manager` supports `--force` to force overwrite an existing version and `--skip-build` if you know that the release artifacts are up to date. If uploads succeed but the release fails afterwards `--skip-upload` can also be used. These flags are primarily used for testing and development purposes; for example if you encounter a network error after a build has succeeded you could use:

```
cargo run --bin=release-manager-- --force --skip-build
```

Be aware force overwriting can cause a checksum mismatch when Cloudfront serves a stale executable version so you should invalidate the Cloudfront distribution.

## Versioning

Multiple versions installed by `uvm` are accessed via the shim executable `uwe-shim` and `upm-shim` which are installed into the installation `bin` directory and determine the version to execute.

They search the current directory and parents for a `.uwe-version` files containing a valid semver which if present is used otherwise they defer to the default version (selected using `uvm use`).

Because `uwe` can also accept paths to projects other than the current working directory and the shim executables have no knowledge of these project path arguments; it must also check whether a switch is needed once it has received a project path. This incurs additional overhead so the search for local versions in this situation should only happen when the project path is not equal to the current working directory as we know that if the project path is the current working directory the shim should already have resolved any local version file.

Running `uvm ls` should also mark an installation as comming from a version file; it can call `release::find_local_version()` to try to find a local file.

## Uninstall

To remove an installation run `cargo run --bin=uvm -- uninstall`.

## Install

To test an installation using the quick install script:

```
curl https://release.uwe.app/install.sh | sh
```

## Plugins

Plugin publishing is restricted to those that have access to the s3 bucket and the `runtime` repository; to publish plugins during the alpha and beta phases certain environment variables need to be set:

```
export UPM_PUBLISH="$HOME/path/to/registry/folder"
export UPM_PUBLISH_REPO="$HOME/path/to/repo"
export UPM_PUBLISH_PROFILE="..."
export UPM_PUBLISH_REGION="ap-southeast-1"
export UPM_PUBLISH_BUCKET="..."
```

It is **required** to set the `pushurl` and `push` refspec:

```
[remote "origin"]
  url = git@github.com:uwe-app/runtime.git
  fetch = +refs/heads/*:refs/remotes/origin/*
  pushurl = git@github.com:uwe-app/runtime.git
  push = refs/heads/main:refs/heads/main
```

## Cross Compiling

To cross-compile from Linux for MacOs the [osxcross][] library is required and you must add the `bin` directory to your `PATH`; see `.cargo/config` and `Makefile` for more details.

## Preferences

This component is currently in limbo but may be restored in the future.

## Notes

Additional information some of which may be obsolete in [NOTES](/NOTES.md).

[runtime]: https://github.com/uwe-app/runtime
[osxcross]: https://github.com/tpoechtrager/osxcross
