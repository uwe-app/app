SITE_ROOT = "../website"

all: init site-release

clean:
	@rm -rf ./target

init-newcss:
	@rm -rf ./build/init-newcss
	@cargo run -- init ./build/init-newcss style/newcss
	@cargo run -- build ./build/init-newcss

init-newcss-open: init-newcss
	@(cd ./build/init-newcss && cargo run -- build --live)

init: init-newcss

help:
	@cargo run -- --help > $(SITE_ROOT)/site/help.txt

site:
	@cargo run -- $(SITE_ROOT)/ --force

site-live:
	@cargo run -- $(SITE_ROOT)/ --live --force

site-release: install help

dist: site-release
	@ht $(SITE_ROOT)/ --release --force --tag=dist
	@rm -f $(SITE_ROOT)/build/hypertext-preview.zip
	@(cd $(SITE_ROOT)/build && zip -r hypertext-preview.zip dist/*)

fmt:
	@cargo fmt

build-release:
	@cargo build --release

copy-release:
	@rm -rf $(SITE_ROOT)/site/resources/files/ht-gnu-linux-x86_64
	@mkdir -p $(SITE_ROOT)/site/resources/files/ht-gnu-linux-x86_64
	@cp -f target/release/ht $(SITE_ROOT)/site/resources/files/ht-gnu-linux-x86_64/ht

copy-release-darwin:
	@rm -rf $(SITE_ROOT)/site/resources/files/ht-darwin-x86_64
	@mkdir -p $(SITE_ROOT)/site/resources/files/ht-darwin-x86_64
	@cp -f target/release/ht $(SITE_ROOT)/site/resources/files/ht-darwin-x86_64/ht

release: build-release copy-release
release-darwin: build-release copy-release-darwin

check:
	@cargo check

install: release
	@cp -f target/release/ht $(HOME)/bin

install-darwin: release-darwin
	@cp -f target/release/ht $(HOME)/bin

.PHONY: all site site-release checksum clean install
