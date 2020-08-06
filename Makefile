# Windows_NT on XP, 2000, 7, Vista, 10 etc.
ifeq ($(OS),Windows_NT) 
    HOST_OS := windows
else
    HOST_OS := $(strip $(shell uname | tr A-Z a-z))
endif

ifeq ($(HOST_OS),darwin)
	HOST_OS = macos
endif

INSTALLER_BIN = hypertext-installer
BUNDLER_BIN = ht-bundle
RELEASE_ROOT = ../release
RELEASE_REPO = $(RELEASE_ROOT)/$(HOST_OS)

VERSION_INFO := $(shell cargo run -- --version)
VERSION := $(subst hypertext ,,$(VERSION_INFO))
VERSION_TAG := "v$(VERSION)"
VERSION_FILE = $(RELEASE_REPO)/version.toml

SITE_ROOT = ../website
SITE_RELEASE := $(SITE_ROOT)/site/resources/files/$(HOST_OS)

all: init site-release

init-newcss:
	@rm -rf ./build/init-newcss
	@cargo run -- init ./build/init-newcss style/newcss
	@cargo run -- build ./build/init-newcss

init-newcss-open: init-newcss
	@(cd ./build/init-newcss && cargo run -- build --live)

init: init-newcss

build-release:
	@cargo build --release --bin=ht

installer:
	@(cd components/installer && cargo build --release --bin=$(INSTALLER_BIN))
	@mkdir -p $(SITE_RELEASE)
	@cp -fv target/release/$(INSTALLER_BIN) $(SITE_RELEASE)/$(INSTALLER_BIN)

bundler:
	@(cd components/extras/bundle && cargo build --release --bin=$(BUNDLER_BIN))
	@cp -f target/release/$(BUNDLER_BIN) $(HOME)/.hypertext/bin

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
	@(cd $(RELEASE_REPO) && git add . && git commit -m "Update release to $(VERSION_TAG)." || true)
	@(cd $(RELEASE_REPO) && git tag -f $(VERSION_TAG) && git push origin master --tags --force)

install: build-release
	@mkdir -p $(HOME)/.hypertext/bin
	@cp -f target/release/ht $(HOME)/.hypertext/bin

build-osx:
	export PATH=/usr/local/osx-ndk-x86/bin:$(PATH)
	export PKG_CONFIG_ALLOW_CROSS=1
	@cargo build --target=x86_64-apple-darwin --release

.PHONY: all site-release install release
