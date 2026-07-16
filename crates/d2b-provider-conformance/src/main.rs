use std::{env, error::Error, process::ExitCode, sync::Arc};

use d2b_provider_sdk::{
    contracts::identity::ProviderType,
    toolkit::{FakeProvider, Fixture, check_provider_conformance},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let arguments: Vec<_> = env::args().collect();
    if !matches!(arguments.as_slice(), [_, command] if command == "self-test") {
        eprintln!("usage: d2b-provider-conformance self-test");
        return ExitCode::FAILURE;
    }
    match run_self_test().await {
        Ok(count) => {
            println!("provider conformance self-test passed for {count} axes");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("provider conformance self-test failed: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run_self_test() -> Result<usize, Box<dyn Error>> {
    for (ordinal, provider_type) in ProviderType::ALL.into_iter().enumerate() {
        let fixture = Fixture::new(provider_type, ordinal)?;
        let instance = Arc::new(FakeProvider::new(fixture.clone())).instance();
        check_provider_conformance(&instance, &fixture).await?;
    }
    Ok(ProviderType::ALL.len())
}

#[cfg(test)]
mod tests {
    use super::run_self_test;

    #[tokio::test]
    async fn every_provider_axis_passes_canonical_conformance() {
        assert_eq!(run_self_test().await.ok(), Some(11));
    }
}
