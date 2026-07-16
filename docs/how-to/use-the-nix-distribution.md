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

GitHub releases also contain
`d2b-provider-toolkit-<version>-x86_64-linux-nix-closure.tar.gz`. This is a
Nix-only binary-cache closure, not a standalone Linux tarball. Verify the
release checksums, unpack it, and run its `import.sh`:

```console
sha256sum --check SHA256SUMS
tar -xzf d2b-provider-toolkit-0.1.0-x86_64-linux-nix-closure.tar.gz
sudo ./d2b-provider-toolkit-0.1.0-x86_64-linux-nix-closure/import.sh
```

The cache is unsigned because there is no release signing key. The importer
requires Nix with the `nix-command` feature and must run as root or a user in
Nix's `trusted-users`; it explicitly uses `--no-check-sigs`. Verify
`SHA256SUMS` first using the copy from the official GitHub release over
authenticated HTTPS. This detects corruption but, because the checksum ships on
the same unsigned release channel, does not defend against compromise of that
channel. Nix still verifies the content-addressed NAR hashes while the importer
copies the complete closure and prints its toolkit store path.
