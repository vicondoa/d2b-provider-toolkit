# Conformance

Conformance has three distinct entrypoints.

## Distribution self-test

```console
cargo run --quiet -p d2b-provider-conformance -- self-test
```

This runs the canonical `FakeProvider` through
`check_provider_conformance` for all provider axes. It proves that the pinned
runtime, contracts, fixtures, and toolkit agree. It does not certify a
third-party provider.

The equivalent Nix command is:

```console
nix run .#conformance
```

## Provider implementation test

Each provider must pass its real `ProviderInstance` and matching canonical
`Fixture` to:

```rust
d2b_provider_sdk::toolkit::check_provider_conformance(
    &instance,
    &fixture,
)
.await
```

The descriptor, target, placement, generations, capabilities, health, and
axis-specific inspection result must be exact. Tests must also cover every
advertised mutation, cancellation, deadline, ambiguous completion, adoption,
lease, and shutdown behavior that generic conformance cannot infer.

The shipped template's exact implementation entrypoint is:

```console
cargo test -p d2b-provider-template \
  template_passes_canonical_read_only_conformance
```

## Source conformance

```console
cargo run --quiet -p d2b-provider-source -- verify
```

This validates 157 canonical files, each file digest, six source-group
fingerprints, the checked-out Git revision when metadata is available, the
inventory snapshot digest, and the complete provider distribution fingerprint.
`scripts/check-distribution-policy.sh` independently compares the inventory with
every Git-listed package file and Cargo metadata target.

CI runs all three entrypoints plus formatting, Clippy, workspace tests, the
zero-work Azure example, and `nix flake check`.
