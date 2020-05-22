all: release

clean:
	@rm -rf ./target

help:
	@cargo run -- --help > site/help.txt

site:
	@rm -rf ./build/debug
	@cargo run -- --clean-url

output:
	@ht --release 2>&1 | tee site/output.txt

site-release: install help output digest

example: install
	@ht example/layout --tag=layout-example
	@ht example/draft --tag=draft-example

fmt:
	@cargo fmt

digest:
	@sha256sum site/files/* > sha256.txt

build:
	@cargo build

build-release:
	@rm -rf ./build/debug
	@cargo build --release

copy-release:
	@cp -f target/release/ht site/files/ht-gnu-linux-x86_64

release: build-release copy-release digest

check:
	@cargo check

install: release
	@cp -f target/release/ht $(HOME)/bin

.PHONY: all site site-release checksum clean install
