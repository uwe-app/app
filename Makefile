all: release

clean:
	@rm -rf ./target

clean-site:
	@rm -rf ./build

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
	@cp -f target/release/hypertext $(HOME)/bin

.PHONY: all, clean, install
