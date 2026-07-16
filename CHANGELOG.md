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

### Fixed

- Release binaries now ship as a checksummed, complete Nix closure with an
  import script instead of a misleading standalone Linux `bin`/`share` archive.

[Unreleased]: https://github.com/vicondoa/d2b-provider-toolkit/compare/v0.1.0...HEAD
