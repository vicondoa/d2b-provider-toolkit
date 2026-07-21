# SDK surface

`d2b-provider-sdk` is a distribution facade, not a protocol implementation.
Its default feature set is empty.

| Path | Canonical owner | Exposed purpose |
| --- | --- | --- |
| `d2b_provider_sdk::toolkit` | `d2b-provider-toolkit` | Adapter, generated service server, registration, fixtures, values, redaction, conformance |
| `d2b_provider_sdk::runtime` | `d2b-provider` | Provider traits, instances, registries, clocks, authenticated RPC proxy |
| `d2b_provider_sdk::contracts::provider` | `d2b-contracts/v2-services` | Provider DTOs and axis traits |
| `d2b_provider_sdk::contracts::identity` | `d2b-contracts/v2-services` dependency | Typed provider, realm, workload, and role identities |
| `d2b_provider_sdk::contracts::component_session` | `d2b-contracts/v2-services` dependency | Placement roles and session contract values |
| `d2b_provider_sdk::session` | `d2b-session` | `ComponentSessionDriver`, `SessionDriverHandle`, and `OwnedAttachment` only |

The facade does not depend on or re-export `d2b-client`,
`d2b-session-unix`, daemon endpoint discovery, socket bootstrap, generated
client convenience APIs, legacy public JSON, broker contracts, or guest
contracts.

The facade and canonical `d2b-provider-toolkit` both select the exact
`d2b-contracts/v2-services` distribution profile. Default features remain
disabled for every canonical dependency.

All crates in the canonical SDK closure and this distribution are
non-publishable. Supported consumption paths are an exact GitHub release source
artifact, a recursive checkout, or the Nix source archive.
