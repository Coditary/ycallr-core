.PHONY: build build-all build-release test test-wasm test-all coverage coverage-html lint fmt fmt-check clean check doc ci all release-check release-check-full

all: build

build:
	cargo build

build-all:
	cargo build --all-features

build-release:
	cargo build --release --all-features

test:
	cargo test --all-features

test-wasm:
	wasm-pack test --node --lib --features wasm

test-all: test test-wasm

coverage:
	cargo tarpaulin --all-features --exclude-files src/wasm.rs --fail-under 85 -- --test-threads 1

coverage-html:
	cargo tarpaulin --all-features --exclude-files src/wasm.rs --fail-under 85 --html -- --test-threads 1

lint:
	cargo clippy --all-features -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

clean:
	cargo clean

check:
	cargo check --all-features

doc:
	cargo doc --all-features

ci: fmt-check lint test coverage

release-check:
	./scripts/check-release.sh

release-check-full:
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make release-check-full VERSION=0.1.2"; \
		exit 1; \
	fi
	./scripts/check-release.sh --full $(VERSION)
