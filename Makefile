.PHONY: build run test bench docker clean fmt lint

build:
	cargo build --release

run:
	cargo run --release

test:
	cargo test -- --test-threads=4

bench:
	cargo bench

docker:
	docker build -t qps-core:latest .

clean:
	cargo clean

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings
