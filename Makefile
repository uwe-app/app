all: release

clean:
	@rm -rf ./target

clean-build:
	@rm -rf ./build

run: clean-build
	@mkdir build
	@RUST_BACKTRACE=1 cargo run -- --exclude=.*\.txt$

build:
	@cargo build

release-darwin:
	@cargo build --release --target=x86_64-apple-darwin

release:
	@cargo build --release

check:
	@cargo check

install: release
	@cp -f target/release/hypertext $(HOME)/bin

.PHONY: all, clean, install
