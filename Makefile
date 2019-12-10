CRATE_NAME:=near-old-stations

.PHONY: all
all: build check

.PHONY: build
build: soft-clean
	cargo build

.PHONY: check
check: soft-clean
	cargo fmt -- --check
	cargo test
	cargo clippy -- -D warnings

.PHONY: soft-clean
soft-clean:
	cargo clean -p $(CRATE_NAME)

.PHONY: clean
clean:
	cargo clean
