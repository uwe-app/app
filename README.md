# Hyper Text

Repositories must be siblings:

```
.
├── blueprint
├── documentation
├── hypertext
├── release
├── standalone
└── website

7 directories, 0 files
⚡
```

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

## Linux

Ensure you have a recent version of `llvm`, I cloned the repo and built from source to get:

```
clang version 12.0.0 (https://github.com/llvm/llvm-project b904324788a8446791dbfbfd9c716644dbac283e)
Target: x86_64-unknown-linux-gnu
```

Which gets us the faster `lld` linker by using these build commands:

```
git clone https://github.com/llvm/llvm-project llvm-project
cd llvm-project
mkdir build
cd build
cmake -G Ninja -DCMAKE_BUILD_TYPE=Release -DLLVM_ENABLE_PROJECTS="clang;lld" -DCMAKE_INSTALL_PREFIX=/usr/local ../llvm
cmake --build .
```

Then I put the `build/bin` directory in `PATH` so that upgrading llvm is a pull and re-compile.

In order to resolve the linker correctly we also need a modern version of GCC, most distros come with really old versions so it is a good idea to build [from source](https://gcc.gnu.org/install/configure.html):

```
git clone git://gcc.gnu.org/git/gcc.git
mkdir gcc-build
../gcc/configure --disable-multilib
make 
sudo make install
```

Be sure to symlink `cc` to `gcc` wherever the new installation is and prefer it in `PATH` and everything should compile ok.

## Cross-compiling

I used these resources to build for OSX from Linux:

* https://github.com/tpoechtrager/osxcross
* https://www.reddit.com/r/rust/comments/6rxoty/tutorial_cross_compiling_from_linux_for_osx/

## Release

To prepare a release for the current platform run the release task:

```
make release
```

The release task will:

1) Build the executable for the current platform
2) Copy it to the release repository bin folder
3) Commit and push the release repository
4) Create a tag with the release version
5) Push the tags

Note that if the release tag already exists it is overwritten.

## Installer

To run the installer locally:

```
cargo run --bin=hypertext-installer --
```

To build the installer and copy the files to the website:

```
make installer
```

## Search

To build the search library you should first install [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) then you can build the wasm file and javascript bindings:

```
cd components/search
make wasm-prod
```

To generate the test data files to `components/search/test/assets` run `make test-data`.

Which generates some `*.st` files in `test/assets` and you should now have enough data to run the tests with `cargo test`.

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
