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

/// Explicit asynchronous authority for substrate effects.
///
/// Implementations receive only canonical operation values. They must acquire
/// credentials, broker capabilities, paths, and other authority from their
/// composition owner rather than ambient process state.
pub trait SubstrateEffects: Send + Sync {
    /// Observe substrate state without applying a mutation.
    fn check<'a>(
        &'a self,
        context: &'a ProviderCallContext<'a>,
        request: &'a ProviderOperationRequest,
    ) -> ProviderFuture<'a, ProviderObservation>;

    /// Plan a substrate remediation without applying it.
    fn plan_remediation<'a>(
        &'a self,
        context: &'a ProviderCallContext<'a>,
        request: &'a ProviderOperationRequest,
    ) -> ProviderFuture<'a, ProviderPlan>;

    /// Apply a previously authorized canonical remediation plan.
    fn apply<'a>(
        &'a self,
        context: &'a ProviderCallContext<'a>,
        request: &'a ProviderPlan,
    ) -> ProviderFuture<'a, MutationReceipt>;
}

/// Substrate provider scaffold, read-only unless effects are supplied.
pub struct InspectOnlySubstrate {
    descriptor: ProviderDescriptor,
    clock: Arc<dyn ProviderClock>,
    effects: Option<Arc<dyn SubstrateEffects>>,
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
        Ok(Self {
            descriptor,
            clock,
            effects: None,
        })
    }

    /// Build the scaffold with explicitly supplied asynchronous effects.
    ///
    /// Supplying a port does not make the provider production-ready; the caller
    /// still owns durable idempotency, cancellation, deadline, and ambiguity
    /// behavior plus focused conformance tests.
    pub fn with_effects(
        descriptor: ProviderDescriptor,
        clock: Arc<dyn ProviderClock>,
        effects: Arc<dyn SubstrateEffects>,
    ) -> Result<Self, TemplateError> {
        let mut provider = Self::new(descriptor, clock)?;
        provider.effects = Some(effects);
        Ok(provider)
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
        request: &'a ProviderOperationRequest,
    ) -> ProviderFuture<'a, ProviderObservation> {
        if let Some(effects) = &self.effects {
            return effects.check(context, request);
        }
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
        request: &'a ProviderOperationRequest,
    ) -> ProviderFuture<'a, ProviderPlan> {
        match &self.effects {
            Some(effects) => effects.plan_remediation(context, request),
            None => self.unavailable(context),
        }
    }

    fn apply<'a>(
        &'a self,
        context: &'a ProviderCallContext<'a>,
        request: &'a ProviderPlan,
    ) -> ProviderFuture<'a, MutationReceipt> {
        match &self.effects {
            Some(effects) => effects.apply(context, request),
            None => self.unavailable(context),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use d2b_provider_sdk::{
        contracts::{
            identity::ProviderType,
            provider::{
                ImplementationId, MutationReceipt, ProviderCallContext, ProviderFailure,
                ProviderFailureKind, ProviderFuture, ProviderHealthReason, ProviderMethod,
                ProviderObservation, ProviderOperationRequest, ProviderPlan, ProviderRemediation,
                RetryClass, SubstrateProvider,
            },
        },
        toolkit::{DeterministicClock, Fixture, check_provider_conformance},
    };

    use super::{InspectOnlySubstrate, SubstrateEffects};

    struct RecordingEffects {
        calls: Arc<AtomicUsize>,
        now_unix_ms: u64,
    }

    impl RecordingEffects {
        fn unavailable(&self, context: &ProviderCallContext<'_>) -> ProviderFailure {
            ProviderFailure {
                kind: ProviderFailureKind::Unavailable,
                retry: RetryClass::Never,
                provider_type: ProviderType::Substrate,
                binding: context.operation.binding(),
                correlation_id: context.operation.correlation_id.clone(),
                occurred_at_unix_ms: self.now_unix_ms,
                reason: ProviderHealthReason::ProviderDegraded,
                remediation: ProviderRemediation::RetryBounded,
            }
        }
    }

    impl SubstrateEffects for RecordingEffects {
        fn check<'a>(
            &'a self,
            context: &'a ProviderCallContext<'a>,
            _request: &'a ProviderOperationRequest,
        ) -> ProviderFuture<'a, ProviderObservation> {
            self.calls.fetch_add(1, Ordering::AcqRel);
            let failure = self.unavailable(context);
            Box::pin(async move { Err(failure) })
        }

        fn plan_remediation<'a>(
            &'a self,
            context: &'a ProviderCallContext<'a>,
            _request: &'a ProviderOperationRequest,
        ) -> ProviderFuture<'a, ProviderPlan> {
            self.calls.fetch_add(1, Ordering::AcqRel);
            let failure = self.unavailable(context);
            Box::pin(async move { Err(failure) })
        }

        fn apply<'a>(
            &'a self,
            _context: &'a ProviderCallContext<'a>,
            _request: &'a ProviderPlan,
        ) -> ProviderFuture<'a, MutationReceipt> {
            Box::pin(async { panic!("apply is not exercised by this port test") })
        }
    }

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

    #[tokio::test]
    async fn effect_authority_is_injected_through_the_async_port() {
        let mut fixture = Fixture::new(ProviderType::Substrate, 0).expect("canonical fixture");
        fixture.descriptor.implementation_id =
            ImplementationId::parse("substrate-template").expect("static implementation id");
        let clock = Arc::new(DeterministicClock::new(fixture.now_unix_ms));
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = InspectOnlySubstrate::with_effects(
            fixture.descriptor.clone(),
            clock,
            Arc::new(RecordingEffects {
                calls: Arc::clone(&calls),
                now_unix_ms: fixture.now_unix_ms,
            }),
        )
        .expect("valid template descriptor");
        let check_operation = fixture
            .operation(ProviderMethod::SubstrateCheck)
            .expect("canonical operation");
        let check_context = fixture.call_context(&check_operation);
        let check_request = fixture
            .request(ProviderMethod::SubstrateCheck)
            .expect("canonical request");
        assert!(matches!(
            SubstrateProvider::check(&provider, &check_context, &check_request).await,
            Err(ProviderFailure {
                kind: ProviderFailureKind::Unavailable,
                ..
            })
        ));

        let operation = fixture
            .operation(ProviderMethod::SubstratePlanRemediation)
            .expect("canonical operation");
        let context = fixture.call_context(&operation);
        let request = fixture
            .request(ProviderMethod::SubstratePlanRemediation)
            .expect("canonical request");

        assert!(matches!(
            SubstrateProvider::plan_remediation(&provider, &context, &request).await,
            Err(ProviderFailure {
                kind: ProviderFailureKind::Unavailable,
                ..
            })
        ));
        assert_eq!(calls.load(Ordering::Acquire), 2);
    }
}
