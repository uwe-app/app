# Hyper Text

## Release

To prepare a release for the target platform run the appropriate release make task, eg:

```
make release OS=linux
make release OS=macos
make release OS=windows
```

The release repositories must be in a `release` folder that is a sibling of this repo, eg:

```
release
├── linux
├── macos
└── windows
```

The release task will:

1) Build the executable for the current platform
2) Copy it to the release repository bin folder
3) Commit and push the release repository
4) Create a tag with the release version
5) Push the tags

Note that if the release tag already exists it is overwritten.

## Cargo Bundle

To create bundles for various platforms install latest cargo-bundle:

```
cargo install cargo-bundle --git https://github.com/burtonageo/cargo-bundle
```

Then run `cargo bundle` or for a release build `cargo bundle --release`.

## SSL

For libgit2 support (`git2` crate) the SSL development package is required. For Ubunut/Mint I installed with `sudo apt-get install libssl-dev`.

For MacOS try this:

```
brew update && brew upgrade
brew install openssl
```

Try to run `brew link --force openssl` and it will error refusing to overwrite the system openssl installation, a command is provided to add the path to your shell RC file.

Use that command to prefer the new openssl and open a new terminal window and run:

```
openssl version -a
```

You should now be able to compile with the SSL dependency.

[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[handlebars]: https://github.com/sunng87/handlebars-rust
[mdbook]: https://github.com/rust-lang/mdBook
