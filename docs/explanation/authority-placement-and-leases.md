# Authority, placement, and leases

Provider type, authority, placement, ownership, and lease state are separate
canonical contracts. Treating any one as an alias for another creates a
privilege escalation or cleanup bug.

## Authority

`ProviderDescriptor.authority` declares exactly one provider axis. Runtime
authority also records process, cgroup, network, user-namespace, persistent
identity, and device-mediation posture. A provider implements only operations
for that axis and advertises only the capabilities it can complete with its
actual authority.

Cloud ownership must be split by resource. Infrastructure may own a VM resource
without owning workload deployment. Runtime may own a workload without owning
the underlying VM. Transport authentication establishes a transport peer; it
never grants d2b lifecycle authorization.

## Placement

`ProviderDescriptor.placement` is the sole realm placement authority:

- trusted first-party in-process placement is for code composed into the owning
  controller;
- provider-agent placement binds a realm, workload, role, endpoint role,
  service package, and agent generation;
- user-agent placement binds behavior to the owning user-side agent.

Provider ID, provider generation, placement generation, endpoint role, and
service package must agree before serving. Do not infer placement from an
endpoint path, peer address, display name, cloud tenant, or relay identity.

External effects are asynchronous injected ports over canonical values.
Provider code receives broker, credential, path, and transport capabilities
from its composition owner; ambient filesystem paths, environment variables,
well-known sockets, and process-global clients are not authority.

## Handles and ownership

Provider handles are opaque, typed, generation-bound evidence. They preserve
realm/workload scope, provider identity, configuration fingerprint, operation
binding, resource generation, owner, and optional expiration. Consumers must
not parse a handle into a host path or cloud credential.

Ownership transfer follows the canonical state machine and epoch. An ambiguous
transfer remains ambiguous until observation; it is never repaired by changing
the recorded owner locally.

## Credential leases

Credential providers issue bounded opaque leases to an exact consumer provider
and placement. Credential material remains inside its owning provider or user
agent. A lease crossing the session boundary carries typed evidence, not a
secret value.

Validate the credential provider, consumer provider, realm/workload scope,
generation, transfer policy, issuance, expiry, and revocation state on every
use. Refresh and revoke are provider operations. Expiry, disconnect, or
uncertain revocation must not become silent success.

Azure examples in this distribution intentionally acquire no lease and perform
no SDK call.
