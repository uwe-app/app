all: release

clean: clean-site
	@rm -rf ./target

clean-site:
	@rm -rf ./build

help:
	@cargo run -- --help > site/template/help.md.hbs

output:
	@ht > site/template/output.md.hbs 2>&1

site: clean-site
	@mkdir build
	@cargo run -- --clean-url

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

.PHONY: all, clean, install
