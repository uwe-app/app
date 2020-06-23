SITE_ROOT = ../website
DOCS_ROOT = ../documentation

ifeq ($(OS),Windows_NT)     # is Windows_NT on XP, 2000, 7, Vista, 10...
    HOST_OS := windows
else
    HOST_OS := $(strip $(shell uname | tr A-Z a-z))
endif

ifeq ($(HOST_OS),darwin)
	HOST_OS = macos
endif

INSTALLER_BIN = hypertext-installer
RELEASE_ROOT = ../release
RELEASE_REPO = $(RELEASE_ROOT)/$(HOST_OS)

VERSION_INFO := $(shell cargo run -- --version)
VERSION := $(subst hypertext ,,$(VERSION_INFO))
VERSION_TAG := "v$(VERSION)"
VERSION_FILE = $(RELEASE_REPO)/version.toml

SITE_RELEASE := $(SITE_ROOT)/site/resources/files/$(HOST_OS)

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

docs:
	@cargo run -- $(SITE_ROOT) --release --force --tag=docs
	@rm -rf $(DOCS_ROOT)/docs
	@cp -rf $(SITE_ROOT)/build/docs $(DOCS_ROOT)
	@rm $(DOCS_ROOT)/docs/files
	@(cd $(DOCS_ROOT) && git add . && git commit -m "Update docs." && git push origin master)

website-dist:
	@cargo run -- $(SITE_ROOT)/ --release --force --tag=dist
	@rm -f $(SITE_ROOT)/build/hypertext-preview.zip
	@(cd $(SITE_ROOT)/build && zip -r hypertext-preview.zip dist/*)

fmt:
	@cargo fmt

build-release:
	@cargo build --release

release-installer:
	@cargo build --release --bin=$(INSTALLER_BIN)
	@mkdir -p $(SITE_RELEASE)
	@cp -fv target/release/$(INSTALLER_BIN) $(SITE_RELEASE)/$(INSTALLER_BIN)

info:
	@echo $(HOST_OS)
	@echo $(VERSION_INFO)
	@echo $(VERSION)
	@echo $(VERSION_TAG)
	@echo $(VERSION_FILE)
	@echo $(RELEASE_REPO)
	@echo $(SITE_RELEASE)

current:
	@printf "" > $(VERSION_FILE)
	@echo "version = \"$(VERSION)\"" >> $(VERSION_FILE)

release: build-release current
	@cp -f target/release/ht $(RELEASE_REPO)/bin/ht
	@(cd $(RELEASE_REPO) && git add . && git commit -m "Update release." || true)
	@(cd $(RELEASE_REPO) && git tag -f $(VERSION_TAG) && git push origin master --tags --force)

check:
	@cargo check

install: release
	@cp -f target/release/ht $(HOME)/bin

install-darwin: release-darwin
	@cp -f target/release/ht $(HOME)/bin

.PHONY: all site site-release checksum clean install
