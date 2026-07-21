use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("source verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args_os().skip(1);
    if arguments.next().as_deref() != Some("verify".as_ref()) {
        return Err(usage());
    }

    let mut source = PathBuf::from("canonical/d2b");
    let mut artifacts = PathBuf::from("contract");
    let mut inventory = PathBuf::from("pins/toolkit-source-contract.json");
    let mut pin = PathBuf::from("pins/d2b-provider-source.json");

    while let Some(argument) = arguments.next() {
        let value = arguments.next().ok_or_else(|| {
            format!(
                "missing value after {}; {}",
                argument.to_string_lossy(),
                usage()
            )
        })?;
        match argument.to_str() {
            Some("--source") => source = PathBuf::from(value),
            Some("--artifacts") => artifacts = PathBuf::from(value),
            Some("--inventory") => inventory = PathBuf::from(value),
            Some("--pin") => pin = PathBuf::from(value),
            _ => return Err(usage()),
        }
    }

    let verified = d2b_provider_source::verify(&source, &artifacts, &inventory, &pin)?;
    println!(
        "verified {} files at {} (inventory {}, fingerprint {})",
        verified.file_count,
        verified.source_revision,
        verified.inventory_revision,
        verified.distribution_fingerprint
    );
    Ok(())
}

fn usage() -> String {
    "usage: d2b-provider-source verify [--source PATH] [--artifacts PATH] [--inventory PATH] [--pin PATH]".to_owned()
}
