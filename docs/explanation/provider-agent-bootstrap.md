# Provider-agent bootstrap remains fail-closed

The canonical provider runtime and session substrate exist, and the control and
edge service APIs are content-frozen. The exact distribution source contract
still pins the landed provider foundation, while endpoint acquisition,
registration, and process composition remain owned by a later integrated
runtime release. This repository must not manufacture that missing authority.

`d2b_provider_sdk::bootstrap::bootstrap_provider_agent` is a closed,
asynchronous seam. Its success type is `Infallible`; it returns
`RuntimeIntegrationUnavailable` without discovery, registration, credentials,
socket access, network access, or other I/O.

This prevents examples from normalizing a guessed endpoint, ambient credential,
direct daemon socket, or fake successful registration. Provider service
construction remains available when an owning component supplies an already
authenticated canonical `SessionDriverHandle`.

`templates/d2b-provider-template/examples/provider_agent_unavailable.rs`
compiles the seam and exits with status 69 after receiving the unavailable
result. It cannot produce an endpoint or a successful bootstrap.

When integrated runtime composition lands, bootstrap work must consume its
canonical route, endpoint, and registration effects, retain Tokio cancellation
and deadline semantics, add end-to-end conformance, update the immutable source
inventory, and remove this unavailable seam in one reviewed compatibility
change.
