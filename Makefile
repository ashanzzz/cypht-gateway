.PHONY: check test run version-check archive

check: version-check
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

run:
	cargo run -p gatewayd

version-check:
	python3 scripts/check-version.py

archive:
	./scripts/source-archive.sh
