//! Read-only provider-author template.
//!
//! The template satisfies the substrate descriptor's required capability shape,
//! but mutation methods fail closed until an author supplies real authority and
//! durable behavior. It must never be registered as production-available.

#![forbid(unsafe_code)]

use std::{error::Error, fmt, sync::Arc};

use d2b_provider_sdk::{
    contracts::{
        identity::ProviderType,
        provider::{
            AdoptionState, MAX_SAFE_JSON_INTEGER, MutationReceipt, ObservationReason,
            ObservedLifecycleState, Provider, ProviderCallContext, ProviderCapability,
            ProviderCapabilitySet, ProviderContractError, ProviderDescriptor, ProviderFailure,
            ProviderFailureKind, ProviderFuture, ProviderHealth, ProviderHealthReason,
            ProviderHealthState, ProviderMethod, ProviderObservation, ProviderOperationRequest,
            ProviderPlan, ProviderRemediation, RetryClass, SubstrateProvider,
        },
    },
    runtime::{ProviderClock, ProviderInstance},
    toolkit::ProviderValues,
};

/// Template construction failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateError {
    /// The descriptor is not a valid substrate-provider descriptor.
    InvalidDescriptor,
    /// The descriptor does not carry the complete required substrate shape.
    CapabilityMismatch,
}

impl fmt::Display for TemplateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidDescriptor => "template requires a valid substrate-provider descriptor",
            Self::CapabilityMismatch => "template requires the canonical substrate capability set",
        })
    }
}

impl Error for TemplateError {}

/// Read-only substrate provider scaffold.
pub struct InspectOnlySubstrate {
    descriptor: ProviderDescriptor,
    clock: Arc<dyn ProviderClock>,
}

impl fmt::Debug for InspectOnlySubstrate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InspectOnlySubstrate")
            .field("generation", &self.descriptor.registry_generation)
            .finish_non_exhaustive()
    }
}

impl InspectOnlySubstrate {
    /// Build the scaffold from an integrity-checked descriptor and clock.
    pub fn new(
        descriptor: ProviderDescriptor,
        clock: Arc<dyn ProviderClock>,
    ) -> Result<Self, TemplateError> {
        descriptor
            .validate()
            .map_err(|_| TemplateError::InvalidDescriptor)?;
        if descriptor.provider_type() != ProviderType::Substrate {
            return Err(TemplateError::InvalidDescriptor);
        }
        let expected = ProviderCapabilitySet::new(vec![
            ProviderCapability(ProviderMethod::SubstrateCheck),
            ProviderCapability(ProviderMethod::SubstratePlanRemediation),
            ProviderCapability(ProviderMethod::SubstrateApply),
        ])
        .map_err(|_| TemplateError::InvalidDescriptor)?;
        if descriptor.capabilities != expected {
            return Err(TemplateError::CapabilityMismatch);
        }
        Ok(Self { descriptor, clock })
    }

    /// Wrap the scaffold as the canonical runtime instance type.
    pub fn instance(self: Arc<Self>) -> ProviderInstance {
        ProviderInstance::Substrate(self)
    }

    fn now_unix_ms(&self) -> u64 {
        self.clock.now_unix_ms()
    }

    fn failure(
        &self,
        context: &ProviderCallContext<'_>,
        kind: ProviderFailureKind,
        reason: ProviderHealthReason,
        remediation: ProviderRemediation,
    ) -> ProviderFailure {
        ProviderFailure {
            kind,
            retry: RetryClass::Never,
            provider_type: ProviderType::Substrate,
            binding: context.operation.binding(),
            correlation_id: context.operation.correlation_id.clone(),
            occurred_at_unix_ms: self.now_unix_ms().min(MAX_SAFE_JSON_INTEGER),
            reason,
            remediation,
        }
    }

    fn unavailable<'a, T>(&'a self, context: &'a ProviderCallContext<'a>) -> ProviderFuture<'a, T>
    where
        T: Send + 'a,
    {
        let failure = self.failure(
            context,
            ProviderFailureKind::Unavailable,
            ProviderHealthReason::ProviderDegraded,
            ProviderRemediation::RetryBounded,
        );
        Box::pin(async move { Err(failure) })
    }

    fn values(&self) -> Result<ProviderValues, ProviderContractError> {
        ProviderValues::new(&self.descriptor, self.now_unix_ms())
    }
}

impl Provider for InspectOnlySubstrate {
    fn descriptor(&self) -> ProviderDescriptor {
        self.descriptor.clone()
    }

    fn health<'a>(
        &'a self,
        context: &'a ProviderCallContext<'a>,
    ) -> ProviderFuture<'a, ProviderHealth> {
        let result = self
            .values()
            .and_then(|values| {
                values.health(
                    ProviderHealthState::Degraded,
                    ProviderHealthReason::ProviderDegraded,
                    ProviderRemediation::InspectProvider,
                )
            })
            .map_err(|_| {
                self.failure(
                    context,
                    ProviderFailureKind::InvariantViolation,
                    ProviderHealthReason::ConfigurationMismatch,
                    ProviderRemediation::RepairConfiguration,
                )
            });
        Box::pin(async move { result })
    }
}

impl SubstrateProvider for InspectOnlySubstrate {
    fn capabilities(&self) -> ProviderCapabilitySet {
        self.descriptor.capabilities.clone()
    }

    fn check<'a>(
        &'a self,
        context: &'a ProviderCallContext<'a>,
        _request: &'a ProviderOperationRequest,
    ) -> ProviderFuture<'a, ProviderObservation> {
        let result = self
            .values()
            .and_then(|values| {
                values.observation(
                    context.operation,
                    None,
                    ObservedLifecycleState::Unknown,
                    AdoptionState::NotAttempted,
                    ObservationReason::MissingEvidence,
                    ProviderHealthState::Degraded,
                    ProviderHealthReason::ProviderDegraded,
                    ProviderRemediation::InspectProvider,
                )
            })
            .map_err(|_| {
                self.failure(
                    context,
                    ProviderFailureKind::InvariantViolation,
                    ProviderHealthReason::ConfigurationMismatch,
                    ProviderRemediation::RepairConfiguration,
                )
            });
        Box::pin(async move { result })
    }

    fn plan_remediation<'a>(
        &'a self,
        context: &'a ProviderCallContext<'a>,
        _request: &'a ProviderOperationRequest,
    ) -> ProviderFuture<'a, ProviderPlan> {
        self.unavailable(context)
    }

    fn apply<'a>(
        &'a self,
        context: &'a ProviderCallContext<'a>,
        _request: &'a ProviderPlan,
    ) -> ProviderFuture<'a, MutationReceipt> {
        self.unavailable(context)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use d2b_provider_sdk::{
        contracts::{identity::ProviderType, provider::ImplementationId},
        toolkit::{DeterministicClock, Fixture, check_provider_conformance},
    };

    use super::InspectOnlySubstrate;

    #[tokio::test]
    async fn template_passes_canonical_read_only_conformance() {
        let mut fixture = Fixture::new(ProviderType::Substrate, 0).expect("canonical fixture");
        fixture.descriptor.implementation_id =
            ImplementationId::parse("substrate-template").expect("static implementation id");
        let clock = Arc::new(DeterministicClock::new(fixture.now_unix_ms));
        let provider = Arc::new(
            InspectOnlySubstrate::new(fixture.descriptor.clone(), clock)
                .expect("valid template descriptor"),
        )
        .instance();

        check_provider_conformance(&provider, &fixture)
            .await
            .expect("template conformance");
    }
}
