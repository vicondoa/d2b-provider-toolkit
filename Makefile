.PHONY: check clippy conformance flake fmt policy source test

check: fmt clippy test policy source conformance flake

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
	cargo test --workspace --all-targets --all-features

policy:
	bash scripts/check-distribution-policy.sh

source:
	cargo run --quiet -p d2b-provider-source -- verify

conformance:
	cargo run --quiet -p d2b-provider-conformance -- self-test
	cargo run --quiet -p d2b-provider-azure-fake

flake:
	nix flake check --print-build-logs
