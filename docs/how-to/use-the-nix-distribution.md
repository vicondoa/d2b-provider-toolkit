# Use the Nix distribution

Add the toolkit as a flake input:

```nix
{
  inputs.d2b-provider-toolkit.url =
    "github:vicondoa/d2b-provider-toolkit/v0.1.0";
}
```

Available outputs on Linux are:

| Output | Contents |
| --- | --- |
| `packages.<system>.d2b-provider-toolkit` | Conformance/source binaries, docs, templates, pins |
| `packages.<system>.sourceArchive` | Reproducible source archive with canonical d2b source populated |
| `packages.<system>.contractDocs` | Canonical contract reference snapshot |
| `apps.<system>.conformance` | Provider distribution self-test |
| `apps.<system>.source-check` | Source verifier for an unpacked source tree |
| `devShells.<system>.default` | Cargo, Rust, Clippy, rustfmt, Git, and jq |

Run:

```console
nix run github:vicondoa/d2b-provider-toolkit/v0.1.0#conformance
nix build github:vicondoa/d2b-provider-toolkit/v0.1.0#sourceArchive
```

The ordinary GitHub repository source archive does not recursively embed Git
submodules. For Cargo path dependencies, use the release source artifact,
`sourceArchive` output, or a recursive Git clone. Each contains the canonical
d2b source at the path expected by the workspace.

The Nix build fetches d2b independently at the revision in `flake.lock`,
overlays the audited public contract artifacts, and then runs the same source
fingerprint verifier as local development. A source revision, artifact, or
inventory mismatch fails the build.
