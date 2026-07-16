# d2b provider toolkit

GitHub/flake distribution for the canonical d2b 2.0 provider SDK. It adds
provider-author templates, conformance commands, documentation, and Nix release
packaging without becoming a second protocol owner.

## Status

The SDK is pinned to canonical `vicondoa/d2b` revision
`7e94327951d30913a1a6e0e7a47d4a24b462deff`. The complete provider distribution
fingerprint is
`75bebbafe99d2ba65e5ce80bc44e644b5d073b7045d5058cbb0fb1f4c539a8f6`.

Live provider-agent endpoint discovery and registration are unavailable until
the canonical core-control services reach content freeze. The compiled
bootstrap seam returns an explicit unavailable error and cannot report fake
success.

Azure material is fake-only and performs no cloud work. This repository has no
Azure SDK, credential, or live network dependency.

## What is distributed

- `d2b-provider-sdk`: a narrow facade over canonical `d2b-provider-toolkit`,
  `d2b-provider`, `d2b-contracts/v2-provider`, and required session driver
  types;
- `d2b-provider-conformance`: the exact distribution self-test entrypoint;
- `d2b-provider-source`: file, source-group, revision, and distribution drift
  verification;
- `templates/d2b-provider-template`: a compiling, fail-closed provider scaffold;
- `examples/d2b-provider-azure-fake`: zero-work fake SDK example;
- Nix packages for the binaries, canonical contract docs, a reproducible source
  archive, and an x86_64-linux Nix closure bundle.

There are no copied wire DTOs, protobuf definitions, identifier types, frame
codecs, redaction implementations, or fake-provider internals. The canonical
crates remain `publish = false`; this distribution is delivered through GitHub
release source artifacts and flake/path dependencies only.

Release binaries are not standalone Linux executables: their ELF interpreter
and shared-library search paths are in `/nix/store`. The binary release asset is
therefore explicitly Nix-only and contains a complete binary-cache closure plus
an import script. See
[Release artifacts](docs/reference/release-artifacts.md).

## Get started

Clone with the pinned canonical source:

```console
git clone --recurse-submodules \
  https://github.com/vicondoa/d2b-provider-toolkit.git
cd d2b-provider-toolkit
make check
```

Consume the SDK from a checkout or release source archive:

```toml
[dependencies]
d2b-provider-sdk = {
  path = "../d2b-provider-toolkit/crates/d2b-provider-sdk",
  default-features = false,
}
```

Provider crates use base-first names:
`d2b-provider-<axis>-<implementation>`. Async implementations use Tokio and
the canonical provider futures; do not introduce a second runtime bridge into
the protocol layer.

## Exact checks

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo run --quiet -p d2b-provider-source -- verify
cargo run --quiet -p d2b-provider-conformance -- self-test
cargo run --quiet -p d2b-provider-azure-fake
nix flake check --print-build-logs
```

See:

- [Start a provider](docs/how-to/start-a-provider.md)
- [Use the Nix distribution](docs/how-to/use-the-nix-distribution.md)
- [SDK surface](docs/reference/sdk-surface.md)
- [Conformance contract](docs/reference/conformance.md)
- [Source pin and drift policy](docs/reference/source-pin.md)
- [Release artifacts](docs/reference/release-artifacts.md)
- [Authority, placement, and leases](docs/explanation/authority-placement-and-leases.md)
- [Redaction boundary](docs/explanation/redaction-boundary.md)
- [Deferred bootstrap](docs/explanation/provider-agent-bootstrap.md)

## License

Apache-2.0. Canonical d2b sources and contract artifacts retain their upstream
license and provenance.
