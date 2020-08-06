#!/usr/bin/env sh

export GIT_USE_NSEC=""
export PATH=$HOME/git/2.external/osxcross/target/bin:$PATH
export PKG_CONFIG_ALLOW_CROSS=1
cargo build --target=x86_64-apple-darwin --release
