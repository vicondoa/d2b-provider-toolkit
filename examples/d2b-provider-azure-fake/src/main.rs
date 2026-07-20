use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use d2b_provider_sdk::{
    bootstrap::{ProviderAgentBootstrapUnavailable, bootstrap_provider_agent},
    contracts::identity::ProviderType,
    toolkit::{FakeProvider, Fixture, check_provider_conformance},
};

#[derive(Debug, Default)]
struct FakeAzureSdk {
    calls: AtomicUsize,
}

impl FakeAzureSdk {
    fn call_count(&self) -> usize {
        self.calls.load(Ordering::Acquire)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let sdk = FakeAzureSdk::default();
    let fixture = Fixture::new(ProviderType::Infrastructure, 0)?;
    let instance = Arc::new(FakeProvider::new(fixture.clone())).instance();
    check_provider_conformance(&instance, &fixture).await?;

    assert_eq!(
        bootstrap_provider_agent().await,
        Err(ProviderAgentBootstrapUnavailable::RuntimeIntegrationUnavailable)
    );
    assert_eq!(sdk.call_count(), 0);
    println!("fake Azure SDK example passed without cloud or bootstrap work");
    Ok(())
}
