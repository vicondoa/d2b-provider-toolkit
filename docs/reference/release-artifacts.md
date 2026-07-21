# Release artifacts

Each release publishes two archives and a `SHA256SUMS` file:

| Artifact | Support contract |
| --- | --- |
| `d2b-provider-toolkit-<version>-source.tar.gz` | Portable source distribution with canonical path dependencies populated |
| `d2b-provider-toolkit-<version>-x86_64-linux-nix-closure.tar.gz` | Nix-only x86_64-linux binary closure |

The binary artifact is not a standalone Linux distribution. Nix builds the
toolkit with an ELF interpreter, runpath, and shared libraries in `/nix/store`.
Archiving only `bin` and `share` would leave executables that cannot run on an
ordinary Filesystem Hierarchy Standard system.

The Nix-only archive contains:

- `cache/`, a Nix binary cache containing every path in the toolkit's recursive
  runtime closure;
- `closure-paths`, the sorted closure manifest;
- `ROOT_PATH`, the toolkit output store path;
- `import.sh`, which imports that root and its dependencies with `nix copy`;
- this support-contract document.

Before publishing, release automation verifies every ELF interpreter, needed
library declaration, and runpath; checks that every closure manifest path has a
matching binary-cache record; validates archive layout; and verifies all
release checksums.

The file cache is intentionally unsigned: this repository has no secure release
signing key. Nix therefore rejects it under default signature policy. The
importer requires root or a user named by Nix's `trusted-users` setting and uses
`--no-check-sigs` explicitly. It does not weaken daemon policy for untrusted
users.

Download both the archive and `SHA256SUMS` from the official GitHub release over
authenticated HTTPS, then run `sha256sum --check SHA256SUMS` before extraction.
The checksum detects corruption and binds the archive to that release metadata;
because it is distributed on the same unsigned release channel, it does not
protect against compromise of that channel or repository. Nix's NAR hashes
still verify the imported closure contents.

Consumers must have Nix with `nix-command` enabled. After checksum verification,
unpack the archive and run `import.sh` as root or a trusted Nix user. The
importer percent-encodes its absolute cache path before constructing the
`file://` URI, including non-ASCII bytes and URI delimiters. It prints the
toolkit store path; binaries are under its `bin` directory. No non-Nix runtime
support is claimed.
