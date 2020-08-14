#!/usr/bin/env sh

export PATH=$HOME/git/2.external/osxcross/target/bin:$PATH
export PKG_CONFIG_ALLOW_CROSS=1
export CC=o64-clang
export CXX=o64-clang++
export LIBZ_SYS_STATIC=1
#export ONIG_STATIC=1
#export RUSTONIG_STATIC_LIBONIG=1
cargo build --target=x86_64-apple-darwin --release
