//! Stable distribution facade for d2b provider authors.
//!
//! The implementation remains in the canonical d2b crates. This crate only
//! gives GitHub/flake consumers one path dependency and deliberately exposes no
//! alternate wire types, codecs, identifiers, or provider implementation.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod bootstrap;

/// Canonical provider runtime traits, registries, and RPC proxies.
pub use d2b_provider as runtime;
/// Canonical provider-agent adapters, fixtures, values, and conformance checks.
pub use d2b_provider_toolkit as toolkit;

/// Canonical serialized contracts enabled through `d2b-contracts/v2-provider`.
pub mod contracts {
    /// Component-session values referenced by provider placement and services.
    pub use d2b_contracts::v2_component_session as component_session;
    /// Canonical realm, workload, role, and provider identity types.
    pub use d2b_contracts::v2_identity as identity;
    /// Canonical provider v2 DTOs and provider traits.
    pub use d2b_contracts::v2_provider as provider;
}

/// The narrow session surface required by the provider service server.
///
/// Socket discovery, daemon endpoint acquisition, and client-side routing are
/// intentionally absent. The core-control API does not yet own a frozen
/// provider-agent bootstrap contract.
pub mod session {
    pub use d2b_session::{ComponentSessionDriver, OwnedAttachment, SessionDriverHandle};
}

/// Full canonical d2b source revision packaged by this distribution.
pub const CANONICAL_D2B_REVISION: &str = "9183b45c6505cfd496e5d537bf6376f884fb16c7";

/// Fingerprint of the canonical provider toolkit distribution source set.
pub const CANONICAL_SOURCE_FINGERPRINT: &str =
    "10f4f1c06de0b23afe2c96702c494782065ee2bd8fd96ab95d578fcd640c0b1e";

#[cfg(test)]
mod tests {
    use super::{CANONICAL_D2B_REVISION, CANONICAL_SOURCE_FINGERPRINT};

    #[test]
    fn source_identity_is_full_lowercase_hex() {
        assert_eq!(CANONICAL_D2B_REVISION.len(), 40);
        assert_eq!(CANONICAL_SOURCE_FINGERPRINT.len(), 64);
        assert!(
            CANONICAL_D2B_REVISION
                .bytes()
                .chain(CANONICAL_SOURCE_FINGERPRINT.bytes())
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
    }
}
