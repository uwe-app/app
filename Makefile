#SOURCES = $(wildcard site/**/*.*)
#DEBUG = $(wildcard build/debug/**/*.*)

all: release

clean:
	@rm -rf ./target

help:
	@cargo run -- --help > site/template/help.md.hbs

output:
	@ht > site/template/output.md.hbs 2>&1

site:
	@rm -rf ./build/debug
	@cargo run -- --clean-url

site-release: install help output
	@rm -rf ./build/release
	@ht --release

fmt:
	@cargo fmt

build:
	@cargo build

release:
	@cargo build --release

check:
	@cargo check

install: release
	@cp -f target/release/ht $(HOME)/bin

.PHONY: all site site-release clean install
