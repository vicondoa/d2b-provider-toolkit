.PHONY: check clippy conformance flake fmt policy policy-test release-test source test

check: fmt clippy test policy policy-test source conformance release-test flake

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
	cargo test --workspace --all-targets --all-features

policy:
	bash scripts/check-distribution-policy.sh

policy-test:
	bash tests/check-distribution-policy.sh

source:
	cargo run --quiet -p d2b-provider-source -- verify

conformance:
	cargo run --quiet -p d2b-provider-conformance -- self-test
	cargo run --quiet -p d2b-provider-azure-fake

release-test:
	bash tests/release-packaging.sh

flake:
	nix flake check --print-build-logs
