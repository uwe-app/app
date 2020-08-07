all: wasm-prod

wasm-dev:
	@wasm-pack build --target web --out-name search --dev

wasm-prod:
	@wasm-pack build --target web --out-name search

test-data:
	@cargo run -- --build test/federalist-config/federalist-min.toml

test-data-full:
	@cargo run -- --build test/federalist-config/federalist.toml

.PHONY: all wasm-dev wasm-prod test-data
