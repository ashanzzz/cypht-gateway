.PHONY: check test run run-mcp version-check repo-check frontend-check php-check archive

check: version-check repo-check frontend-check php-check
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

run:
	cargo run -p gatewayd

run-mcp:
	cargo run -p cypht-mcp -- --transport stdio

version-check:
	python3 scripts/check-version.py

repo-check:
	python3 scripts/check-repository.py

frontend-check:
	python3 scripts/check-frontend.py

php-check:
	find cypht-module -name '*.php' -print0 | xargs -0 -n1 php -l

archive:
	./scripts/source-archive.sh
