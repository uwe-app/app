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

MAC_STRIP = x86_64-apple-darwin15-strip

all: init

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

strip-release:
	strip target/release/uwe
	strip target/release/upm
	strip target/release/uvm

compile-release:
	@cargo build --release

build-release: compile-release strip-release

strip-linux-macos-cross:
	$(MAC_STRIP) target/x86_64-apple-darwin/release/uwe
	$(MAC_STRIP) target/x86_64-apple-darwin/release/upm
	$(MAC_STRIP) target/x86_64-apple-darwin/release/uvm

compile-linux-macos-cross:
	@PKG_CONFIG_ALLOW_CROSS=1 \
		LIBZ_SYS_STATIC=1 \
		CC=o64-clang \
		CXX=o64-clang++ \
		cargo build --target=x86_64-apple-darwin --release

build-linux-macos-cross: compile-linux-macos-cross strip-linux-macos-cross

release: build-release build-linux-macos-cross

install: build-release
	@mkdir -p $(HOME)/.uwe/bin
	@cp -f \
		target/release/uwe \
		target/release/upm \
		target/release/uvm \
		target/release/uws \
		$(HOME)/.uwe/bin

.PHONY: all install release
