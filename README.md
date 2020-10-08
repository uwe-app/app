# Universal Web Editor

Repositories must be siblings, for example:

```
.
├── app
├── blog
├── library
├── plugins
├── runtime
├── syntax
└── website
```

* `app`: Main source code (this repository).
* `blog`: Canonical blog website.
* `library`: Additional libraries; search runtime and third-party forks.
* `plugins`: Source code for the plugins.
* `runtime`: Runtime library; syntax highlighting and registry cache etc.
* `syntax`: Builds the syntax highlighting runtime files to the `runtime`.
* `website`: Source code for the project website.

## Releases

A private executable `uwe-publish` performs all the steps for a release.

1) Bump the version in `Cargo.toml`.
2) Publish a new release: `cargo run --bin=uwe-publish`.
3) Commit and push the new release version and checksums in the [runtime][] repository (`releases.json`).

If you need them `uwe-publish` supports `--force` to force overwrite an existing version and `--skip-build` if you know that the release artifacts are up to date. These flags are primarily used for testing and development purposes; for example if you encounter a network error after a build has succeeded you could use:

```
cargo run --bin=uwe-publish -- --force --skip-build
```

## Uninstall

To remove an installation run `cargo run --bin=uvm -- uninstall`.

## Install

To test an installation using the quick install script:

```
curl https://release.uwe.app/install.sh | sh
```

## Plugins

Plugin publishing is restricted to those that have access to the s3 bucket and the registry repository; to publish plugins during the alpha and beta phases certain environment variables need to be set:

```
export AB_PUBLISH="$HOME/path/to/registry/folder"
export AB_PUBLISH_REPO="$HOME/path/to/repo"
export AB_PUBLISH_PROFILE="..."
export AB_PUBLISH_REGION="ap-southeast-1"
export AB_PUBLISH_BUCKET="..."
```

It is **required** to set the `pushurl` and `push` refspec:

```
[remote "origin"]
  url = git@github.com:uwe-app/runtime.git
  fetch = +refs/heads/*:refs/remotes/origin/*
  pushurl = git@github.com:uwe-app/runtime.git
  push = refs/heads/master:refs/heads/master
```

## Cross Compiling

To cross-compile from Linux for MacOs the [osxcross][] library is required and you must add the `bin` directory to your `PATH`; see `.cargo/config` and `Makefile` for more details.

## Preferences

This component is currently in limbo but may be restored in the future.

## Notes

Additional information some of which may be obsolete in [NOTES](/NOTES.md).

[runtime]: https://github.com/uwe-app/runtime
[osxcross]: https://github.com/tpoechtrager/osxcross
