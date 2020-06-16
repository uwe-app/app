# Hyper Text

Super fast, opinionated, site generator combining [pulldown-cmark][], [handlebars][] and [mdbook][].

```
ht
```

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
