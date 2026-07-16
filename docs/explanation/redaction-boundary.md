# Redaction boundary

Redaction is owned by canonical d2b types. The generic provider wrappers are:

```rust
d2b_provider_sdk::toolkit::{Redacted, Secret}
```

`Secret<T>` exposes its value only through an explicit closure or consuming
operation. `Redacted<T>` keeps `Debug` output type-only. Provider, identity,
session, attachment, and target values have their own canonical redacted
`Debug`/`Display` behavior.

Do not create a serializable redaction wrapper, opaque-handle DTO, alternate
error envelope, or convenience wire model. That would become a second protocol
source. Presentation models must remain non-wire and outside authority checks.

Logs, traces, metrics, errors, panic messages, and test snapshots must not
contain:

- credentials, tokens, lease material, or secret configuration;
- opaque handles, operation/idempotency/correlation identifiers, or raw
  requests and responses;
- realm, workload, role, principal, or provider identifiers;
- endpoint paths, cloud resource names, tenant data, or attachment bytes.

Use closed labels for operation class, provider axis, outcome, and canonical
reason/remediation enums. Test redaction with sentinel values and assert their
absence from every formatted error and debug value.
