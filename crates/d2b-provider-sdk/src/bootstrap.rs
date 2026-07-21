//! Closed provider-agent bootstrap seam.
//!
//! Endpoint acquisition and registration remain unavailable until canonical
//! runtime composition lands in a pinned d2b release. This module cannot
//! manufacture a successful endpoint and performs no I/O.

use std::{convert::Infallible, error::Error, fmt};

/// Why this distribution cannot bootstrap a live provider agent yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderAgentBootstrapUnavailable {
    /// Canonical runtime composition is absent from the pinned source release.
    RuntimeIntegrationUnavailable,
}

impl fmt::Display for ProviderAgentBootstrapUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "provider-agent bootstrap is unavailable until canonical runtime integration lands",
        )
    }
}

impl Error for ProviderAgentBootstrapUnavailable {}

/// Returns the explicit unavailable state without performing discovery or I/O.
///
/// `Infallible` makes a fabricated success value impossible. A later
/// distribution may replace this seam only when the canonical runtime owns
/// endpoint acquisition and registration in a pinned release.
pub async fn bootstrap_provider_agent() -> Result<Infallible, ProviderAgentBootstrapUnavailable> {
    Err(ProviderAgentBootstrapUnavailable::RuntimeIntegrationUnavailable)
}

#[cfg(test)]
mod tests {
    use super::{ProviderAgentBootstrapUnavailable, bootstrap_provider_agent};

    #[tokio::test]
    async fn bootstrap_is_closed() {
        assert_eq!(
            bootstrap_provider_agent().await,
            Err(ProviderAgentBootstrapUnavailable::RuntimeIntegrationUnavailable)
        );
    }
}
