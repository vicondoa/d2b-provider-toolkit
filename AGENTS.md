# Contributor guide

This repository distributes the canonical d2b provider SDK. It does not own
provider wire contracts.

## Invariants

- Keep `canonical/d2b`, `flake.lock`, `pins/d2b-provider-source.json`, and the
  canonical contract snapshot on their audited revisions.
- Never copy or redefine wire DTOs, protobuf, identifiers, session framing,
  redaction wrappers, generated bindings, or `FakeProvider` internals.
- Use canonical crates through path dependencies. Registry publication is not
  supported; every crate has `publish = false`.
- Keep default features empty. Add dependencies only to the crate that needs
  them, and keep cloud dependencies out of fake examples.
- Provider implementation crates use
  `d2b-provider-<axis>-<implementation>` names and Tokio async execution.
- Do not implement live provider-agent bootstrap until the canonical
  core-control service contract is content-frozen. The unavailable seam must
  remain fail-closed.
- Never log credentials, lease material, opaque handles, operation identifiers,
  endpoint paths, or raw provider requests/responses.

## Changes

Update `CHANGELOG.md` for code, packaging, or public documentation changes.
Source pin updates must be intentional, review the upstream inventory diff,
refresh canonical contract artifacts, regenerate `flake.lock`, and pass the
source verifier before other tests.

Run:

```console
make check
```

Release tags are `v<workspace-version>`. Release automation creates source and
x86_64-linux archives with checksums; it never publishes to a crate registry.
