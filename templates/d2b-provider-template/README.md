# Provider template

This compiling scaffold implements a read-only substrate provider. The
descriptor carries the substrate contract's required capability shape, while
mutation methods fail with canonical unavailable errors.

Copy the directory, rename the crate to
`d2b-provider-<axis>-<implementation>`, replace the axis deliberately, and keep
default features empty. The descriptor must come from trusted configuration.
Do not register the scaffold as production-available until every required
operation owns durable behavior and focused tests.

Validate the unchanged template with:

```console
cargo test -p d2b-provider-template \
  template_passes_canonical_read_only_conformance
```

Do not add endpoint discovery or registration here while provider-agent
bootstrap remains unavailable.

The compiled bootstrap example is deliberately unsuccessful:

```console
cargo run -p d2b-provider-template --example provider_agent_unavailable
# exits 69 after reporting the canonical unavailable state
```
