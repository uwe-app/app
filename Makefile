all: release

clean:
	@rm -rf ./target

run:
	@cargo run -- test target --ignore=.*\.txt$

build:
	@cargo build

release:
	@cargo build --release

check:
	@cargo check

install: release
	@cp -f target/release/hypertext $(HOME)/bin

.PHONY: all, clean, install
