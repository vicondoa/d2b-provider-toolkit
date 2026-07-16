# Source pin and drift policy

The provider distribution has two immutable provenance points:

| Purpose | Revision |
| --- | --- |
| Canonical d2b source | `9183b45c6505cfd496e5d537bf6376f884fb16c7` |
| Source inventory and public contract artifacts | `b1f2c13a196004f5a0fb999808d691b0668cf226` |

The inventory confirms that the selected canonical code groups are
byte-identical at the source revision. Public contract artifacts are retained
verbatim under `contract/`. Their snapshot, the canonical source checkout, and the
domain-separated inventory produce:

```text
d2b-provider-toolkit
10f4f1c06de0b23afe2c96702c494782065ee2bd8fd96ab95d578fcd640c0b1e
```

`pins/d2b-provider-source.json` records both revisions, the inventory snapshot
digest, all source-group fingerprints, and the distribution fingerprint.
`canonical/d2b` is a Git submodule at the canonical revision. `flake.nix` and
`flake.lock` fetch that same revision independently for Nix builds.

## Fail-closed checks

`d2b-provider-source verify`:

1. accepts only the canonical repository and fingerprint policy;
2. rejects unsafe inventory paths and non-regular files;
3. verifies the inventory snapshot digest;
4. verifies the submodule revision when Git metadata exists;
5. verifies every selected file digest;
6. recomputes each source-group fingerprint;
7. recomputes the sorted-union distribution fingerprint.

Release archives omit Git metadata, so file and domain-separated distribution
digests remain the authority there.

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
