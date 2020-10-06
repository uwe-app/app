# Windows_NT on XP, 2000, 7, Vista, 10 etc.
ifeq ($(OS),Windows_NT) 
    HOST_OS := windows
else
    HOST_OS := $(strip $(shell uname | tr A-Z a-z))
endif

ifeq ($(HOST_OS),darwin)
	HOST_OS = macos
endif

INSTALLER_BIN = uwe-installer
RELEASE_ROOT = ../release
RELEASE_REPO = $(RELEASE_ROOT)/$(HOST_OS)

VERSION_INFO := $(shell cargo run -- --version)
VERSION := $(subst uwe ,,$(VERSION_INFO))
VERSION_TAG := "v$(VERSION)"
VERSION_FILE = $(RELEASE_REPO)/version.toml

SITE_ROOT = ../website
SITE_RELEASE := $(SITE_ROOT)/site/resources/files/$(HOST_OS)

all: init site-release

docs:
	@cargo doc --open --no-deps --lib --workspace

build-release:
	@cargo build --release --bin=ht

installer:
	@(cd components/installer && cargo build --release --bin=$(INSTALLER_BIN))
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
	@(cd $(RELEASE_REPO) && git add . && git commit -m "Update release to $(VERSION_TAG)." || true)
	@(cd $(RELEASE_REPO) && git tag -f $(VERSION_TAG) && git push origin master --tags --force)

install: build-release
	@mkdir -p $(HOME)/.uwe/bin
	@cp -f target/release/ht $(HOME)/.uwe/bin

.PHONY: all site-release install release
