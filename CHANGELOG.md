# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Canonical d2b provider SDK facade, source pin verification, provider-author
  template, conformance entrypoints, and zero-work fake Azure example.
- Reproducible Nix packages, source archives, CI, and GitHub release
  automation.
- Provider authoring, redaction, authority, placement, lease, bootstrap, and
  source-drift documentation.
- `nix flake check` now includes hermetic `cargo fmt --check` and
  `cargo clippy -D warnings` checks built from the same distribution source
  tree as the packaged toolkit, so formatting and lint regressions are caught
  without a local checkout.

### Changed

- The provider template now accepts explicit asynchronous substrate effect
  ports while retaining fail-closed read-only behavior by default.

### Fixed

- The SDK now selects the canonical provider distribution's exact
  `d2b-contracts/v2-services` feature profile, and policy checks cover canonical
  dependency features, Git-listed package files, Cargo targets, copied wire
  code, and ambient authority.
- Provider-agent bootstrap now reports the actual integrated-runtime blocker
  instead of the completed control-service content freeze.
- Source verification now consumes the package-complete canonical inventory,
  pins the landed d2b revision, and rejects every unlisted file
  below a selected Cargo package root.
- Release binaries now ship as a checksummed, complete Nix closure with an
  import script instead of a misleading standalone Linux `bin`/`share` archive.
- The closure importer now handles unsigned-cache trust explicitly and safely
  imports from extraction paths containing URI delimiters or non-ASCII bytes.
- Trusted Nix users are parsed without pathname expansion, including wildcard,
  group, and exact-user principals.
- The distribution policy check's author-root scans now fail closed on a
  missing/unreadable author root or any `grep` error (exit code >= 2) instead
  of silently treating the error as "no match found".

[Unreleased]: https://github.com/vicondoa/d2b-provider-toolkit/compare/v0.1.0...HEAD
