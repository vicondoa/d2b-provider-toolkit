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

Consumers must have Nix with `nix-command` enabled. After checking
`SHA256SUMS`, unpack the archive and run `import.sh`. The script prints the
toolkit store path; binaries are under its `bin` directory. No non-Nix runtime
support is claimed.
