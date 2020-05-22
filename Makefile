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

example: install
	@ht example/layout --tag=layout-example
	@ht example/draft --tag=draft-example

fmt:
	@cargo fmt

digest:
	@sha256sum site/files/* > site/download/sha256.txt

build:
	@cargo build

build-release:
	@cargo build --release

copy-release:
	@cp -f target/release/ht site/files/ht-gnu-linux-x86_64

release: build-release copy-release digest

check:
	@cargo check

install: release
	@cp -f target/release/ht $(HOME)/bin

.PHONY: all site site-release checksum clean install
