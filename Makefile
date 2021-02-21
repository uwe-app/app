# Windows_NT on XP, 2000, 7, Vista, 10 etc.
ifeq ($(OS),Windows_NT)
    HOST_OS := windows
else
    HOST_OS := $(strip $(shell uname | tr A-Z a-z))
endif

ifeq ($(HOST_OS),darwin)
	HOST_OS = macos
endif

VERSION_INFO := $(shell cargo run -- --version)
VERSION := $(subst uwe ,,$(VERSION_INFO))
VERSION_TAG := "v$(VERSION)"

MAC_STRIP = x86_64-apple-darwin20.2-strip

all: docs install

docs:
	@cargo doc --open --no-deps --lib --workspace

info:
	@echo $(HOST_OS)
	@echo $(VERSION_INFO)
	@echo $(VERSION)
	@echo $(VERSION_TAG)

strip-release:
	strip target/release/uwe
	strip target/release/uwe-shim
	strip target/release/upm
	strip target/release/upm-shim
	strip target/release/uvm

compile-release:
	@cargo build --release

build-release: compile-release strip-release

strip-linux-macos-cross:
	$(MAC_STRIP) target/x86_64-apple-darwin/release/uwe
	$(MAC_STRIP) target/x86_64-apple-darwin/release/uwe-shim
	$(MAC_STRIP) target/x86_64-apple-darwin/release/upm
	$(MAC_STRIP) target/x86_64-apple-darwin/release/upm-shim
	$(MAC_STRIP) target/x86_64-apple-darwin/release/uvm

compile-linux-macos-cross:
	@PKG_CONFIG_ALLOW_CROSS=1 \
		LZMA_API_STATIC=1 \
		WINIT_LINK_COLORSYNC=1 \
		CC=o64-clang \
		CXX=o64-clang++ \
		cargo build --target=x86_64-apple-darwin --release

build-linux-macos-cross: compile-linux-macos-cross strip-linux-macos-cross

release: build-release build-linux-macos-cross

strip-private:
	strip target/release/web-host
	$(MAC_STRIP) target/x86_64-apple-darwin/release/web-host

build-private:
	@cargo build --bin=web-host --release
	@PKG_CONFIG_ALLOW_CROSS=1 \
		LIBZ_SYS_STATIC=1 \
		CC=o64-clang \
		CXX=o64-clang++ \
		cargo build --target=x86_64-apple-darwin --bin=web-host --release

private: build-private strip-private

install: build-release
	@mkdir -p $(HOME)/.uwe/bin
	@cp -f \
		target/release/uwe \
		target/release/upm \
		target/release/uvm \
		$(HOME)/.uwe/bin

.PHONY: all docs install release
