SITE_ROOT = "../website"

all: init example site-release

clean:
	@rm -rf ./target

init-newcss:
	@rm -rf ./build/init-newcss
	@cargo run -- init --template=newcss ./build/init-newcss
	@cargo run -- build ./build/init-newcss/site ./build/init-newcss/build

init-tacit:
	@rm -rf ./build/init-tacit
	@cargo run -- init --template=tacit ./build/init-tacit
	@cargo run -- build ./build/init-tacit/site ./build/init-tacit/build

init-bahunya:
	@rm -rf ./build/init-bahunya
	@cargo run -- init --template=bahunya ./build/init-bahunya
	@cargo run -- build ./build/init-bahunya/site ./build/init-bahunya/build

init-newcss-open: init-newcss
	@(cd ./build/init-newcss && cargo run -- build --live)

init-tacit-open: init-tacit
	@(cd ./build/init-tacit && cargo run -- build --live)

init-bahunya-open: init-bahunya
	@(cd ./build/init-bahunya && cargo run -- build --live)

init: init-newcss init-tacit init-bahunya

help:
	@cargo run -- --help > $(SITE_ROOT)/site/help.txt

site:
	@cargo run -- $(SITE_ROOT)/site $(SITE_ROOT)/build --force

site-live:
	@cargo run -- $(SITE_ROOT)/site $(SITE_ROOT)/build --live --force

site-release: install help

dist: site-release
	@ht $(SITE_ROOT)/site $(SITE_ROOT)/build --release --force --index-links --tag=hypertext-preview
	@rm -f $(SITE_ROOT)/build/hypertext-preview.zip
	@(cd $(SITE_ROOT)/build && zip -r hypertext-preview.zip hypertext-preview/*)

example: install
	@ht example/layout --tag=layout-example
	@ht example/draft --tag=draft-example

fmt:
	@cargo fmt

build-release:
	@cargo build --release

copy-release:
	@rm -rf $(SITE_ROOT)/site/files/ht-gnu-linux-x86_64
	@mkdir -p $(SITE_ROOT)/site/files/ht-gnu-linux-x86_64
	@cp -f target/release/ht $(SITE_ROOT)/site/files/ht-gnu-linux-x86_64/ht

copy-release-darwin:
	@rm -rf $(SITE_ROOT)/site/files/ht-darwin-x86_64
	@mkdir -p $(SITE_ROOT)/site/files/ht-darwin-x86_64
	@cp -f target/release/ht $(SITE_ROOT)/site/files/ht-darwin-x86_64/ht

release: build-release copy-release
release-darwin: build-release copy-release-darwin

check:
	@cargo check

install: release
	@cp -f target/release/ht $(HOME)/bin

install-darwin: release-darwin
	@cp -f target/release/ht $(HOME)/bin

.PHONY: all site site-release checksum clean install
