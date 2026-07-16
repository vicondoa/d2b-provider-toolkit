//! Verification for the immutable canonical source packaged by this
//! distribution.

#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
    process::Command,
};

use serde::Deserialize;
use sha2::{Digest, Sha256};

const EXPECTED_REPOSITORY: &str = "https://github.com/vicondoa/d2b";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourcePin {
    schema_version: u32,
    repository: String,
    source_revision: String,
    inventory_revision: String,
    inventory_sha256: String,
    distribution_id: String,
    distribution_fingerprint: String,
    source_groups: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Inventory {
    fingerprint_policy: FingerprintPolicy,
    source_groups: Vec<SourceGroup>,
    distributions: Vec<Distribution>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintPolicy {
    algorithm: String,
    digest_encoding: String,
    distribution_domain: String,
    length_encoding: String,
    path_encoding: String,
    source_group_domain: String,
}

#[derive(Debug, Deserialize)]
struct SourceGroup {
    id: String,
    fingerprint: String,
    files: Vec<SourceFile>,
}

#[derive(Debug, Deserialize)]
struct SourceFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Distribution {
    id: String,
    fingerprint: String,
    source_groups: Vec<String>,
}

/// Successful verification summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    /// Canonical source revision declared by the pin.
    pub source_revision: String,
    /// Revision from which the source inventory was snapshotted.
    pub inventory_revision: String,
    /// Number of unique files covered by the distribution fingerprint.
    pub file_count: usize,
    /// Verified provider distribution fingerprint.
    pub distribution_fingerprint: String,
}

/// Verify the source checkout, canonical artifact snapshot, inventory, and pin.
///
/// When `source` retains Git metadata, its checked-out revision must also match
/// the pin. Release archives omit Git metadata and are authenticated by every
/// file digest plus the domain-separated distribution fingerprint.
pub fn verify(
    source: &Path,
    artifacts: &Path,
    inventory_path: &Path,
    pin_path: &Path,
) -> Result<Verification, String> {
    let pin_bytes = read_regular_file(pin_path, "source pin")?;
    let pin: SourcePin = serde_json::from_slice(&pin_bytes)
        .map_err(|error| format!("source pin is not valid JSON: {error}"))?;
    validate_pin(&pin)?;

    let inventory_bytes = read_regular_file(inventory_path, "source inventory")?;
    require_digest("source inventory", &inventory_bytes, &pin.inventory_sha256)?;
    let inventory: Inventory = serde_json::from_slice(&inventory_bytes)
        .map_err(|error| format!("source inventory is not valid JSON: {error}"))?;
    validate_policy(&inventory.fingerprint_policy)?;

    verify_git_revision_when_present(source, &pin.source_revision)?;

    let distribution = inventory
        .distributions
        .iter()
        .find(|distribution| distribution.id == pin.distribution_id)
        .ok_or_else(|| "source inventory does not contain the pinned distribution".to_owned())?;
    if distribution.fingerprint != pin.distribution_fingerprint {
        return Err("pin and inventory distribution fingerprints differ".to_owned());
    }

    let expected_groups: BTreeSet<_> = distribution.source_groups.iter().cloned().collect();
    let pinned_groups: BTreeSet<_> = pin.source_groups.keys().cloned().collect();
    if expected_groups != pinned_groups {
        return Err("pin and inventory distribution source groups differ".to_owned());
    }

    let mut distribution_files = BTreeMap::<String, Vec<u8>>::new();
    for group_id in &distribution.source_groups {
        let group = inventory
            .source_groups
            .iter()
            .find(|group| &group.id == group_id)
            .ok_or_else(|| format!("source inventory is missing group {group_id}"))?;
        let pinned_fingerprint = pin
            .source_groups
            .get(group_id)
            .ok_or_else(|| format!("source pin is missing group {group_id}"))?;
        if &group.fingerprint != pinned_fingerprint {
            return Err(format!(
                "pin and inventory fingerprints differ for group {group_id}"
            ));
        }

        let mut group_files = BTreeMap::new();
        for entry in &group.files {
            validate_relative_path(&entry.path)?;
            let root = if group.id == "public-contract-artifacts" {
                artifacts
            } else {
                source
            };
            let bytes = read_regular_file(&root.join(&entry.path), &entry.path)?;
            require_digest(&entry.path, &bytes, &entry.sha256)?;
            if group_files
                .insert(entry.path.clone(), bytes.clone())
                .is_some()
            {
                return Err(format!(
                    "duplicate path in group {}: {}",
                    group.id, entry.path
                ));
            }
            match distribution_files.insert(entry.path.clone(), bytes) {
                Some(previous) if previous != group_files[&entry.path] => {
                    return Err(format!(
                        "distribution path has conflicting bytes: {}",
                        entry.path
                    ));
                }
                _ => {}
            }
        }

        let actual = fingerprint(
            &inventory.fingerprint_policy.source_group_domain,
            &group.id,
            &group_files,
        );
        if actual != group.fingerprint {
            return Err(format!("source group fingerprint mismatch: {}", group.id));
        }
    }

    let actual_distribution = fingerprint(
        &inventory.fingerprint_policy.distribution_domain,
        &distribution.id,
        &distribution_files,
    );
    if actual_distribution != distribution.fingerprint {
        return Err("provider distribution fingerprint mismatch".to_owned());
    }

    Ok(Verification {
        source_revision: pin.source_revision,
        inventory_revision: pin.inventory_revision,
        file_count: distribution_files.len(),
        distribution_fingerprint: actual_distribution,
    })
}

fn validate_pin(pin: &SourcePin) -> Result<(), String> {
    if pin.schema_version != 1 {
        return Err("unsupported source pin schema version".to_owned());
    }
    if pin.repository != EXPECTED_REPOSITORY {
        return Err("source pin repository is not canonical".to_owned());
    }
    require_hex("source revision", &pin.source_revision, 40)?;
    require_hex("inventory revision", &pin.inventory_revision, 40)?;
    require_hex("inventory digest", &pin.inventory_sha256, 64)?;
    require_hex(
        "distribution fingerprint",
        &pin.distribution_fingerprint,
        64,
    )?;
    if pin.distribution_id != "d2b-provider-toolkit" {
        return Err("source pin selects the wrong distribution".to_owned());
    }
    Ok(())
}

fn validate_policy(policy: &FingerprintPolicy) -> Result<(), String> {
    if policy.algorithm != "sha256"
        || policy.digest_encoding != "lowercase-hex"
        || policy.length_encoding != "u64-big-endian"
        || policy.path_encoding != "utf-8"
        || policy.source_group_domain != "d2b-toolkit-source-group-v1"
        || policy.distribution_domain != "d2b-toolkit-distribution-v1"
    {
        return Err("unsupported source fingerprint policy".to_owned());
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("inventory contains an unsafe path: {path}"));
    }
    Ok(())
}

fn read_regular_file(path: &Path, label: &str) -> Result<Vec<u8>, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("cannot inspect {label}: {error}"))?;
    if !metadata.file_type().is_file() {
        return Err(format!("{label} is not a regular file"));
    }
    fs::read(path).map_err(|error| format!("cannot read {label}: {error}"))
}

fn require_digest(label: &str, bytes: &[u8], expected: &str) -> Result<(), String> {
    let actual = hex_digest(bytes);
    if actual != expected {
        return Err(format!("SHA-256 mismatch for {label}"));
    }
    Ok(())
}

fn require_hex(label: &str, value: &str, length: usize) -> Result<(), String> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{label} must be {length} lowercase hexadecimal bytes"
        ));
    }
    Ok(())
}

fn verify_git_revision_when_present(source: &Path, expected: &str) -> Result<(), String> {
    if !source.join(".git").exists() {
        return Ok(());
    }
    let output = Command::new("git")
        .args(["-C"])
        .arg(source)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("cannot run git for canonical source: {error}"))?;
    if !output.status.success() {
        return Err("cannot resolve canonical source revision".to_owned());
    }
    let actual = String::from_utf8(output.stdout)
        .map_err(|_| "canonical source revision is not UTF-8".to_owned())?;
    if actual.trim() != expected {
        return Err("canonical source checkout is at the wrong revision".to_owned());
    }
    Ok(())
}

fn fingerprint(domain: &str, id: &str, files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(id.as_bytes());
    hasher.update([0]);
    for (path, bytes) in files {
        let path = path.as_bytes();
        hasher.update(u64::try_from(path.len()).unwrap_or(u64::MAX).to_be_bytes());
        hasher.update(path);
        hasher.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
        hasher.update(bytes);
    }
    hex_bytes(&hasher.finalize())
}

fn hex_digest(bytes: &[u8]) -> String {
    hex_bytes(&Sha256::digest(bytes))
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{fingerprint, require_hex, validate_relative_path};

    #[test]
    fn fingerprint_is_ordered_and_domain_separated() {
        let files = BTreeMap::from([
            ("a".to_owned(), b"first".to_vec()),
            ("z".to_owned(), b"last".to_vec()),
        ]);
        assert_eq!(
            fingerprint("domain", "distribution", &files),
            "a0a44c32baa5eaeb165c53d8ba710583811731f22a1de104c056e87228a163d6"
        );
        assert_ne!(
            fingerprint("other", "distribution", &files),
            fingerprint("domain", "distribution", &files)
        );
    }

    #[test]
    fn unsafe_paths_and_noncanonical_hex_are_rejected() {
        assert!(validate_relative_path("../source").is_err());
        assert!(validate_relative_path("/source").is_err());
        assert!(require_hex("digest", "AA", 2).is_err());
        assert!(require_hex("digest", "aa", 2).is_ok());
    }
}
