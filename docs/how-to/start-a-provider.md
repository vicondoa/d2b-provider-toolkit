# Start a provider

This procedure creates a provider implementation without introducing another
copy of the d2b protocol.

## 1. Start from the template

Copy `templates/d2b-provider-template` into your repository and rename the
package using:

```text
d2b-provider-<axis>-<implementation>
```

Keep `default = []` and `publish = false`. Depend on
`d2b-provider-sdk` with `default-features = false`.

The shipped template is a read-only substrate provider. Its descriptor has the
complete required substrate capability shape, while remediation methods return
canonical unavailable failures. It must not enter a production registry until
every required method owns durable behavior and focused tests.

## 2. Accept a trusted descriptor

Construct the provider from a descriptor supplied by the integrity-checked d2b
configuration path. Validate it before retaining it. Do not derive realm,
workload, role, provider, or generation identifiers from display names,
environment variables, or cloud responses.

The descriptor's authority and placement must agree with the process that owns
the implementation. A provider agent cannot claim trusted in-process placement,
and transport authentication cannot grant local lifecycle authority.

## 3. Implement with canonical types

Import traits and values through:

```rust
use d2b_provider_sdk::{
    contracts::provider::{Provider, ProviderFuture},
    runtime::ProviderInstance,
    toolkit::ProviderValues,
};
```

Return `ProviderFuture` values driven by Tokio. Preserve operation deadlines,
cancellation, idempotency keys, generations, placement, and owner bindings.
Mutation uncertainty must return the canonical ambiguous outcome and require
observation; it must not be converted to success.

Use `ProviderValues` to build health, plans, handles, observations, receipts,
and failures. Do not define local equivalents.

## 4. Add conformance

Build a `ProviderInstance` from the real implementation and a canonical
`Fixture` whose descriptor and target match it:

```rust
#[tokio::test]
async fn provider_conforms() {
    let fixture = make_fixture();
    let instance = make_provider(&fixture);
    d2b_provider_sdk::toolkit::check_provider_conformance(
        &instance,
        &fixture,
    )
    .await
    .expect("provider conformance");
}
```

Run the provider test and the distribution checks described in
[Conformance](../reference/conformance.md).

## 5. Keep bootstrap closed

Do not discover a daemon socket, invent an endpoint, self-register, or report a
successful provider-agent bootstrap. The owning core-control contract is not
content-frozen. See [Provider-agent bootstrap](../explanation/provider-agent-bootstrap.md).

## 6. Keep cloud examples fake

Use a fake SDK boundary in tests. Assert that unavailable paths issue zero SDK
calls. Do not add Azure credentials, live clients, network calls, or production
capabilities to an example.
