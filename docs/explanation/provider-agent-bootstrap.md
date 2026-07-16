# Provider-agent bootstrap is deferred

The canonical provider runtime and session substrate exist, but the
core-control services that resolve a provider-agent endpoint and register it are
not content-frozen. This distribution therefore cannot define a live bootstrap
example yet.

`d2b_provider_sdk::bootstrap::bootstrap_provider_agent` is a closed,
asynchronous seam. Its success type is `Infallible`; it returns
`CoreControlServicesNotFrozen` without discovery, registration, credentials,
socket access, network access, or other I/O.

This prevents examples from normalizing a guessed endpoint, ambient credential,
direct daemon socket, or fake successful registration. Provider service
construction remains available when an owning component supplies an already
authenticated canonical `SessionDriverHandle`.

`templates/d2b-provider-template/examples/provider_agent_unavailable.rs`
compiles the seam and exits with status 69 after receiving the unavailable
result. It cannot produce an endpoint or a successful bootstrap.

After the core-control content freeze, bootstrap work must consume the
canonical route/endpoint/registration API, retain Tokio cancellation and
deadline semantics, add end-to-end conformance, update the source inventory,
and remove this unavailable seam in one reviewed compatibility change.
