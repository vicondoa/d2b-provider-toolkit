# Source pin and drift policy

The provider distribution has two immutable provenance points:

| Purpose | Revision |
| --- | --- |
| Canonical d2b source | `4018d9c9652bd826c2e6a9abccdcdcafb832d944` |
| Source inventory and public contract artifacts | `c645a769f50b8283c1eddeb12f2a9bf0a1f397bd` |

The inventory confirms that the selected canonical code groups are
byte-identical at the source revision. Public contract artifacts are retained
verbatim under `contract/`. Their snapshot, the canonical source checkout, and the
domain-separated inventory produce:

```text
d2b-provider-toolkit
89f76b9ab63515ecccf46c642676ac5d3c6b4e53bfc642d1dacb69818e3e8588
```

`pins/d2b-provider-source.json` records both revisions, the inventory snapshot
digest, all source-group fingerprints, and the distribution fingerprint.
`canonical/d2b` is a Git submodule at the canonical revision. `flake.nix` and
`flake.lock` fetch that same revision independently for Nix builds.

## Fail-closed checks

`d2b-provider-source verify`:

1. accepts only the canonical repository and fingerprint policy;
2. rejects unsafe inventory paths, symbolic-link traversal, and non-regular
   files;
3. verifies the inventory snapshot digest;
4. verifies the submodule revision and rejects dirty or untracked files when
   Git metadata exists;
5. verifies every selected file digest;
6. recomputes each source-group fingerprint;
7. recomputes the sorted-union distribution fingerprint;
8. enumerates every file below each selected Cargo package root, including
   libraries, binaries, examples, tests, fixtures, protobuf inputs,
   feature-gated modules, custom builds, and local build dependencies, and
   rejects any input omitted from the inventory.

Release archives omit Git metadata, so file and domain-separated distribution
digests plus complete package-source enumeration remain the authority there.
When `package.build` is omitted and no default `build.rs` exists, complete
package enumeration proves that absence. Every default or explicitly selected
custom build script must be inventoried and hashed; local path build
dependencies must also be selected, fully inventoried canonical packages.

## Updating the pin

A pin update is a deliberate compatibility change:

1. select an immutable canonical d2b revision;
2. obtain the source inventory and contract artifacts from its accepted
   coordination revision;
3. review all changed paths and Cargo feature selections;
4. update the submodule, pin JSON, contract snapshot, SDK constants, and flake
   input together;
5. regenerate `flake.lock` and the repository `Cargo.lock`;
6. run the source verifier before compiling;
7. run the full conformance and Nix gates;
8. describe compatibility and migration effects in `CHANGELOG.md`.

Never bless drift by editing a digest alone. If a canonical source group
changes, inspect and adopt the upstream API change or keep the previous pin.
