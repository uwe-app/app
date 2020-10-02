# Hyper Text

Repositories must be siblings, for example:

```
.
├── blog
├── hypertext
├── library
├── plugins
├── release
├── runtime
├── syntax
└── website
```

* `blog`: Canonical blog website.
* `hypertext`: Main source code (this repository).
* `library`: Additional libraries; search runtime and third-party forks.
* `plugins`: Source code for the plugins.
* `release`: Binary releases organized by platform.
* `runtime`: Runtime library; syntax highlighting and registry cache etc.
* `syntax`: Builds the syntax highlighting runtime files to the `runtime`.
* `website`: Source code for the project website.

The release repositories take the following form; they must be in a `release` folder that is a sibling of this repo, eg:

```
release
├── linux
├── macos
└── windows
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
  url = git@github.com:hypertext-live/runtime.git
  fetch = +refs/heads/*:refs/remotes/origin/*
  pushurl = git@github.com:hypertext-live/runtime.git
  push = refs/heads/master:refs/heads/master
```

## Preferences

This component is currently in limbo but may be restored in the future.

## Notes

Additional information some of which may be obsolete in [NOTES](/NOTES.md).
