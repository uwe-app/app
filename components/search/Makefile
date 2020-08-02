all: init

wasm-dev:
	@wasm-pack build --target web --out-name search --dev

wasm-prod:
	@wasm-pack build --target web --out-name search

test-data:
	@cargo run -- --build test/federalist-config/federalist-min.toml 

.PHONY: all wasm-dev wasm-prod test-data
