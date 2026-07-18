.PHONY: all build release check test lint fmt clippy clean run install ci

all: build

build:
	cargo build

release:
	cargo build --release

check:
	cargo check --workspace

test:
	cargo test --workspace

test-verbose:
	cargo test --workspace -- --nocapture

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy -- -D warnings

lint: fmt clippy

clean:
	cargo clean

run:
	cargo run

install:
	cargo install --path .

uninstall:
	cargo uninstall jsscli

ci: fmt-check clippy test

rebuild:
	make release && make install

snap:
	snapcat tests -f markdown -o dev/tests.snapcat.md && snapcat src -f markdown -o dev/src.snapcat.md