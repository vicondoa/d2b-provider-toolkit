use std::process::ExitCode;

use d2b_provider_sdk::bootstrap::bootstrap_provider_agent;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match bootstrap_provider_agent().await {
        Ok(never) => match never {},
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(69)
        }
    }
}
