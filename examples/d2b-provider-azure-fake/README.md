# Fake Azure example

This example proves the fake-only posture:

- canonical fake-provider conformance runs for the infrastructure axis;
- live provider-agent bootstrap returns its closed unavailable state;
- the fake Azure SDK call count remains zero;
- there is no Azure crate, credential, network, or live cloud dependency.

Run:

```console
cargo run --quiet -p d2b-provider-azure-fake
```
