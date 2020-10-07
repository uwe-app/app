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
#RELEASE_ROOT = ../release
#RELEASE_REPO = $(RELEASE_ROOT)/$(HOST_OS)

VERSION_INFO := $(shell cargo run -- --version)
VERSION := $(subst uwe ,,$(VERSION_INFO))
VERSION_TAG := "v$(VERSION)"
VERSION_FILE = $(RELEASE_REPO)/version.toml

SITE_ROOT = ../website
SITE_RELEASE := $(SITE_ROOT)/site/resources/files/$(HOST_OS)

all: init site-release

docs:
	@cargo doc --open --no-deps --lib --workspace

installer:
	@cargo build --release --bin=$(INSTALLER_BIN)
	@mkdir -p $(SITE_RELEASE)
	@cp -fv target/release/$(INSTALLER_BIN) $(SITE_RELEASE)/$(INSTALLER_BIN)

info:
	@echo $(HOST_OS)
	@echo $(VERSION_INFO)
	@echo $(VERSION)
	@echo $(VERSION_TAG)
	@echo $(VERSION_FILE)
	@echo $(SITE_RELEASE)

current:
	@printf "" > $(VERSION_FILE)
	@echo "version = \"$(VERSION)\"" >> $(VERSION_FILE)

build-release:
	@cargo build --release

build-linux-macos-cross:
	@PKG_CONFIG_ALLOW_CROSS=1 \
		LIBZ_SYS_STATIC=1 \
		CC=o64-clang \
		CXX=o64-clang++ \
		cargo build --target=x86_64-apple-darwin --release

install: build-release
	@mkdir -p $(HOME)/.uwe/bin
	@cp -f target/release/uwe target/release/upm $(HOME)/.uwe/bin

.PHONY: all site-release install release
