//! Closed provider-agent bootstrap seam.
//!
//! Endpoint acquisition and registration remain unavailable until the d2b
//! core-control services reach content freeze. This module cannot manufacture a
//! successful endpoint and performs no I/O.

use std::{convert::Infallible, error::Error, fmt};

/// Why this distribution cannot bootstrap a live provider agent yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderAgentBootstrapUnavailable {
    /// The owning core-control endpoint and registration API is not frozen.
    CoreControlServicesNotFrozen,
}

impl fmt::Display for ProviderAgentBootstrapUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "provider-agent bootstrap is unavailable until core-control services are frozen",
        )
    }
}

impl Error for ProviderAgentBootstrapUnavailable {}

/// Returns the explicit unavailable state without performing discovery or I/O.
///
/// `Infallible` makes a fabricated success value impossible. A later
/// distribution may replace this seam only after the canonical core-control
/// service contract is content-frozen.
pub async fn bootstrap_provider_agent() -> Result<Infallible, ProviderAgentBootstrapUnavailable> {
    Err(ProviderAgentBootstrapUnavailable::CoreControlServicesNotFrozen)
}

#[cfg(test)]
mod tests {
    use super::{ProviderAgentBootstrapUnavailable, bootstrap_provider_agent};

    #[tokio::test]
    async fn bootstrap_is_closed() {
        assert_eq!(
            bootstrap_provider_agent().await,
            Err(ProviderAgentBootstrapUnavailable::CoreControlServicesNotFrozen)
        );
    }
}
