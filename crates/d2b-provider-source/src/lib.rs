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
    require_directory_root(source, "canonical source")?;
    require_directory_root(artifacts, "contract artifacts")?;
    let pin_bytes = read_regular_file(pin_path, "source pin")?;
    let pin: SourcePin = serde_json::from_slice(&pin_bytes)
        .map_err(|error| format!("source pin is not valid JSON: {error}"))?;
    validate_pin(&pin)?;

    let inventory_bytes = read_regular_file(inventory_path, "source inventory")?;
    require_digest("source inventory", &inventory_bytes, &pin.inventory_sha256)?;
    let inventory: Inventory = serde_json::from_slice(&inventory_bytes)
        .map_err(|error| format!("source inventory is not valid JSON: {error}"))?;
    validate_policy(&inventory.fingerprint_policy)?;

    verify_git_checkout_when_present(source, &pin.source_revision)?;

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
            let bytes = read_rooted_regular_file(root, &entry.path)?;
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

    verify_cargo_build_inputs(source, distribution_files.keys())?;

    Ok(Verification {
        source_revision: pin.source_revision,
        inventory_revision: pin.inventory_revision,
        file_count: distribution_files.len(),
        distribution_fingerprint: actual_distribution,
    })
}

fn require_directory_root(path: &Path, label: &str) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("cannot inspect {label}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(format!("{label} root is not a safe directory"));
    }
    Ok(())
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

fn read_rooted_regular_file(root: &Path, relative: &str) -> Result<Vec<u8>, String> {
    validate_relative_path(relative)?;
    let mut candidate = root.to_path_buf();
    let components: Vec<_> = Path::new(relative).components().collect();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err(format!("inventory contains an unsafe path: {relative}"));
        };
        candidate.push(component);
        let metadata = fs::symlink_metadata(&candidate)
            .map_err(|error| format!("cannot inspect {relative}: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("{relative} traverses a symbolic link"));
        }
        if index + 1 == components.len() {
            if !metadata.file_type().is_file() {
                return Err(format!("{relative} is not a regular file"));
            }
        } else if !metadata.file_type().is_dir() {
            return Err(format!("{relative} traverses a non-directory"));
        }
    }
    fs::read(candidate).map_err(|error| format!("cannot read {relative}: {error}"))
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

fn verify_git_checkout_when_present(source: &Path, expected: &str) -> Result<(), String> {
    if !source.join(".git").exists() {
        return Ok(());
    }
    let output = run_git(source, &["rev-parse", "--show-toplevel"])?;
    if !output.status.success() {
        return Err("cannot resolve canonical source checkout root".to_owned());
    }
    let checkout_root = String::from_utf8(output.stdout)
        .map_err(|_| "canonical source revision is not UTF-8".to_owned())?;
    let source_root = fs::canonicalize(source)
        .map_err(|error| format!("cannot resolve canonical source path: {error}"))?;
    let git_root = fs::canonicalize(checkout_root.trim())
        .map_err(|error| format!("cannot resolve canonical Git root: {error}"))?;
    if source_root != git_root {
        return Err("canonical source path is not the Git checkout root".to_owned());
    }

    let output = run_git(source, &["rev-parse", "HEAD"])?;
    if !output.status.success() {
        return Err("cannot resolve canonical source revision".to_owned());
    }
    let actual = String::from_utf8(output.stdout)
        .map_err(|_| "canonical source revision is not UTF-8".to_owned())?;
    if actual.trim() != expected {
        return Err("canonical source checkout is at the wrong revision".to_owned());
    }

    let output = run_git(
        source,
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ],
    )?;
    if !output.status.success() {
        return Err("cannot inspect canonical source checkout state".to_owned());
    }
    if !output.stdout.is_empty() {
        return Err("canonical source checkout has dirty or untracked files".to_owned());
    }
    Ok(())
}

fn run_git(source: &Path, arguments: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .args(["-C"])
        .arg(source)
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot run git for canonical source: {error}"))
}

#[derive(Debug, Default)]
struct ManifestAudit {
    build: Option<BuildSetting>,
    build_dependencies: Vec<String>,
    explicit_inputs: Vec<String>,
}

#[derive(Debug)]
enum BuildSetting {
    Disabled,
    Path(String),
}

fn verify_cargo_build_inputs<'a>(
    source: &Path,
    inventory_paths: impl Iterator<Item = &'a String>,
) -> Result<(), String> {
    let inventory: BTreeSet<&str> = inventory_paths.map(String::as_str).collect();
    let manifests: Vec<_> = inventory
        .iter()
        .copied()
        .filter(|path| {
            path.starts_with("packages/")
                && path.ends_with("/Cargo.toml")
                && path.matches('/').count() == 2
        })
        .collect();
    if manifests.is_empty() {
        return Err("distribution contains no canonical Cargo packages".to_owned());
    }

    let manifest_set: BTreeSet<_> = manifests.iter().copied().collect();
    for manifest in manifests {
        let package_dir = manifest
            .strip_suffix("/Cargo.toml")
            .expect("manifest suffix was checked");
        let bytes = read_rooted_regular_file(source, manifest)?;
        let audit = parse_manifest(manifest, &bytes)?;

        let default_build_script = format!("{package_dir}/build.rs");
        let default_build_exists = path_exists_without_symlink(source, &default_build_script)?;
        match &audit.build {
            Some(BuildSetting::Disabled) | None => {}
            Some(BuildSetting::Path(path)) => {
                let input = package_relative_path(package_dir, path, manifest)?;
                require_in_inventory(&inventory, &input, manifest)?;
                read_rooted_regular_file(source, &input)?;
            }
        }

        for input in audit.explicit_inputs {
            let input = package_relative_path(package_dir, &input, manifest)?;
            require_in_inventory(&inventory, &input, manifest)?;
            read_rooted_regular_file(source, &input)?;
        }

        for dependency in audit.build_dependencies {
            let dependency_dir = package_relative_path(package_dir, &dependency, manifest)?;
            let dependency_manifest = format!("{dependency_dir}/Cargo.toml");
            if !manifest_set.contains(dependency_manifest.as_str()) {
                return Err(format!(
                    "{manifest} has an unpinned canonical build dependency: {dependency_manifest}"
                ));
            }
        }

        if default_build_exists {
            require_in_inventory(&inventory, &default_build_script, manifest)?;
        }
        for input in [
            format!("{package_dir}/src/lib.rs"),
            format!("{package_dir}/src/main.rs"),
        ] {
            if path_exists_without_symlink(source, &input)? {
                require_in_inventory(&inventory, &input, manifest)?;
            }
        }
        for directory in ["src/bin", "examples", "benches"] {
            let relative = format!("{package_dir}/{directory}");
            collect_cargo_input_files(source, &relative, &inventory, manifest)?;
        }
    }
    Ok(())
}

fn parse_manifest(label: &str, bytes: &[u8]) -> Result<ManifestAudit, String> {
    let contents = std::str::from_utf8(bytes).map_err(|_| format!("{label} is not valid UTF-8"))?;
    let mut audit = ManifestAudit::default();
    let mut section = "";

    for raw_line in contents.lines() {
        let line = strip_toml_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            section = line.trim_start_matches('[').trim_end_matches(']').trim();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if section == "package" && key == "build" {
            if audit.build.is_some() {
                return Err(format!("{label} declares package.build more than once"));
            }
            audit.build = Some(if value == "false" {
                BuildSetting::Disabled
            } else {
                BuildSetting::Path(parse_toml_string(value, label, "package.build")?)
            });
        } else if matches!(section, "lib" | "bin" | "example" | "test" | "bench") && key == "path" {
            audit
                .explicit_inputs
                .push(parse_toml_string(value, label, "target path")?);
        } else if section.starts_with("build-dependencies.") && key == "path" {
            audit.build_dependencies.push(parse_toml_string(
                value,
                label,
                "build dependency path",
            )?);
        } else if section == "build-dependencies"
            && let Some(path) = inline_table_path(value, label)?
        {
            audit.build_dependencies.push(path);
        }
    }
    Ok(audit)
}

fn strip_toml_comment(line: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in line.bytes().enumerate() {
        if escaped {
            escaped = false;
        } else if byte == b'\\' && quoted {
            escaped = true;
        } else if byte == b'"' {
            quoted = !quoted;
        } else if byte == b'#' && !quoted {
            return &line[..index];
        }
    }
    line
}

fn parse_toml_string(value: &str, label: &str, field: &str) -> Result<String, String> {
    if value.starts_with('"') {
        serde_json::from_str(value).map_err(|_| format!("{label} has an unsupported {field} value"))
    } else if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        Ok(value[1..value.len() - 1].to_owned())
    } else {
        Err(format!("{label} has an unsupported {field} value"))
    }
}

fn inline_table_path(value: &str, label: &str) -> Result<Option<String>, String> {
    let Some(path_index) = value.find("path") else {
        return Ok(None);
    };
    let path_value = value[path_index + "path".len()..]
        .trim_start()
        .strip_prefix('=')
        .ok_or_else(|| format!("{label} has an unsupported build dependency path"))?
        .trim_start();
    let end = quoted_value_end(path_value)
        .ok_or_else(|| format!("{label} has an unsupported build dependency path"))?;
    parse_toml_string(&path_value[..end], label, "build dependency path").map(Some)
}

fn quoted_value_end(value: &str) -> Option<usize> {
    let quote = *value.as_bytes().first()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let mut escaped = false;
    for (index, byte) in value.bytes().enumerate().skip(1) {
        if escaped {
            escaped = false;
        } else if byte == b'\\' && quote == b'"' {
            escaped = true;
        } else if byte == quote {
            return Some(index + 1);
        }
    }
    None
}

fn package_relative_path(package_dir: &str, relative: &str, label: &str) -> Result<String, String> {
    if relative.is_empty() || Path::new(relative).is_absolute() {
        return Err(format!("{label} contains an unsafe Cargo path: {relative}"));
    }
    let mut components: Vec<_> = package_dir.split('/').map(str::to_owned).collect();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(component) => {
                let component = component
                    .to_str()
                    .ok_or_else(|| format!("{label} has a non-UTF-8 Cargo path"))?;
                components.push(component.to_owned());
            }
            Component::CurDir => {}
            Component::ParentDir if components.len() > 1 => {
                components.pop();
            }
            _ => return Err(format!("{label} contains an unsafe Cargo path: {relative}")),
        }
    }
    Ok(components.join("/"))
}

fn path_exists_without_symlink(source: &Path, relative: &str) -> Result<bool, String> {
    let path = source.join(relative);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(format!("{relative} is a symbolic link"))
        }
        Ok(metadata) => Ok(metadata.file_type().is_file()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("cannot inspect {relative}: {error}")),
    }
}

fn require_in_inventory(
    inventory: &BTreeSet<&str>,
    input: &str,
    manifest: &str,
) -> Result<(), String> {
    if inventory.contains(input) {
        Ok(())
    } else {
        Err(format!(
            "{manifest} has an unlisted Cargo build input: {input}"
        ))
    }
}

fn collect_cargo_input_files(
    source: &Path,
    relative: &str,
    inventory: &BTreeSet<&str>,
    manifest: &str,
) -> Result<(), String> {
    let directory = source.join(relative);
    let metadata = match fs::symlink_metadata(&directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("cannot inspect {relative}: {error}")),
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(format!("{relative} is not a safe Cargo input directory"));
    }
    collect_cargo_input_directory(&directory, relative, inventory, manifest)
}

fn collect_cargo_input_directory(
    directory: &Path,
    relative: &str,
    inventory: &BTreeSet<&str>,
    manifest: &str,
) -> Result<(), String> {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .map_err(|error| format!("cannot enumerate {relative}: {error}"))?
        .collect::<Result<_, _>>()
        .map_err(|error| format!("cannot enumerate {relative}: {error}"))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("{relative} contains a non-UTF-8 path"))?;
        let child_relative = format!("{relative}/{name}");
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| format!("cannot inspect {child_relative}: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("{child_relative} is a symbolic link"));
        }
        if metadata.file_type().is_dir() {
            collect_cargo_input_directory(&entry.path(), &child_relative, inventory, manifest)?;
        } else if metadata.file_type().is_file() {
            require_in_inventory(inventory, &child_relative, manifest)?;
        } else {
            return Err(format!("{child_relative} is not a regular Cargo input"));
        }
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
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
    };

    use serde_json::json;

    use super::{fingerprint, hex_digest, require_hex, validate_relative_path, verify};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        source: PathBuf,
        artifacts: PathBuf,
        inventory: PathBuf,
        pin: PathBuf,
        files: BTreeMap<String, Vec<u8>>,
    }

    impl Fixture {
        fn new(manifest: &str, extra_files: &[(&str, &str)]) -> Self {
            Self::with_packages(
                &[
                    ("packages/example/Cargo.toml", manifest),
                    ("packages/example/src/lib.rs", "pub fn example() {}\n"),
                ],
                extra_files,
            )
        }

        fn with_packages(base_files: &[(&str, &str)], extra_files: &[(&str, &str)]) -> Self {
            let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::current_dir()
                .expect("current directory")
                .join("target")
                .join(format!(
                    "d2b-provider-source-test-{}-{id}",
                    std::process::id()
                ));
            if root.exists() {
                fs::remove_dir_all(&root).expect("remove stale fixture");
            }
            let source = root.join("source");
            let artifacts = root.join("artifacts");
            fs::create_dir_all(&source).expect("create source");
            fs::create_dir_all(&artifacts).expect("create artifacts");

            let mut files = BTreeMap::new();
            for (path, contents) in base_files.iter().chain(extra_files.iter()) {
                write_file(&source.join(path), contents.as_bytes());
                files.insert((*path).to_owned(), contents.as_bytes().to_vec());
            }
            let inventory = root.join("inventory.json");
            let pin = root.join("pin.json");
            let fixture = Self {
                root,
                source,
                artifacts,
                inventory,
                pin,
                files,
            };
            fixture.write_contract(&"0".repeat(40));
            fixture
        }

        fn write_contract(&self, revision: &str) {
            let group_fingerprint =
                fingerprint("d2b-toolkit-source-group-v1", "canonical", &self.files);
            let distribution_fingerprint = fingerprint(
                "d2b-toolkit-distribution-v1",
                "d2b-provider-toolkit",
                &self.files,
            );
            let entries: Vec<_> = self
                .files
                .iter()
                .map(|(path, bytes)| json!({"path": path, "sha256": hex_digest(bytes)}))
                .collect();
            let inventory = serde_json::to_vec_pretty(&json!({
                "fingerprintPolicy": {
                    "algorithm": "sha256",
                    "digestEncoding": "lowercase-hex",
                    "distributionDomain": "d2b-toolkit-distribution-v1",
                    "lengthEncoding": "u64-big-endian",
                    "pathEncoding": "utf-8",
                    "sourceGroupDomain": "d2b-toolkit-source-group-v1"
                },
                "sourceGroups": [{
                    "id": "canonical",
                    "fingerprint": group_fingerprint,
                    "files": entries
                }],
                "distributions": [{
                    "id": "d2b-provider-toolkit",
                    "fingerprint": distribution_fingerprint,
                    "sourceGroups": ["canonical"]
                }]
            }))
            .expect("serialize inventory");
            fs::write(&self.inventory, &inventory).expect("write inventory");
            let pin = serde_json::to_vec_pretty(&json!({
                "schemaVersion": 1,
                "repository": "https://github.com/vicondoa/d2b",
                "sourceRevision": revision,
                "inventoryRevision": "1".repeat(40),
                "inventorySha256": hex_digest(&inventory),
                "distributionId": "d2b-provider-toolkit",
                "distributionFingerprint": distribution_fingerprint,
                "sourceGroups": {"canonical": group_fingerprint}
            }))
            .expect("serialize pin");
            fs::write(&self.pin, pin).expect("write pin");
        }

        fn verify(&self) -> Result<super::Verification, String> {
            verify(&self.source, &self.artifacts, &self.inventory, &self.pin)
        }

        fn init_git(&self) {
            git(&self.source, &["init", "--quiet"]);
            git(&self.source, &["config", "user.name", "Verifier Test"]);
            git(
                &self.source,
                &["config", "user.email", "verifier@example.invalid"],
            );
            git(&self.source, &["add", "."]);
            git(&self.source, &["commit", "--quiet", "-m", "fixture"]);
            let revision = git_output(&self.source, &["rev-parse", "HEAD"]);
            self.write_contract(revision.trim());
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            if self.root.exists() {
                fs::remove_dir_all(&self.root).expect("remove fixture");
            }
        }
    }

    fn write_file(path: &Path, contents: &[u8]) {
        fs::create_dir_all(path.parent().expect("file parent")).expect("create file parent");
        fs::write(path, contents).expect("write fixture file");
    }

    fn git(source: &Path, arguments: &[&str]) {
        let status = Command::new("git")
            .args(["-C"])
            .arg(source)
            .args(arguments)
            .status()
            .expect("run git");
        assert!(status.success(), "git command failed: {arguments:?}");
    }

    fn git_output(source: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(["-C"])
            .arg(source)
            .args(arguments)
            .output()
            .expect("run git");
        assert!(output.status.success(), "git command failed: {arguments:?}");
        String::from_utf8(output.stdout).expect("git output UTF-8")
    }

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

    #[test]
    fn clean_archive_with_explicitly_disabled_build_script_is_accepted() {
        let fixture = Fixture::new("[package]\nname = \"example\"\nbuild = false\n", &[]);
        assert!(fixture.verify().is_ok());
    }

    #[test]
    fn clean_archive_with_verified_absent_build_script_is_accepted() {
        let fixture = Fixture::new("[package]\nname = \"example\"\n", &[]);
        assert!(fixture.verify().is_ok());
    }

    #[test]
    fn incomplete_archive_is_rejected() {
        let fixture = Fixture::new("[package]\nname = \"example\"\n", &[]);
        fs::remove_file(fixture.source.join("packages/example/src/lib.rs"))
            .expect("remove inventoried source");
        let error = fixture.verify().expect_err("incomplete archive must fail");
        assert!(error.contains("cannot inspect"), "{error}");
    }

    #[test]
    fn git_checkout_rejects_untracked_and_dirty_files() {
        let untracked = Fixture::new("[package]\nname = \"example\"\nbuild = false\n", &[]);
        untracked.init_git();
        write_file(&untracked.source.join("untracked.rs"), b"fn main() {}\n");
        let error = untracked.verify().expect_err("untracked source must fail");
        assert!(error.contains("dirty or untracked"), "{error}");

        let dirty = Fixture::new("[package]\nname = \"example\"\nbuild = false\n", &[]);
        dirty.init_git();
        write_file(
            &dirty.source.join("packages/example/src/lib.rs"),
            b"pub fn changed() {}\n",
        );
        let error = dirty.verify().expect_err("dirty source must fail");
        assert!(error.contains("dirty or untracked"), "{error}");
    }

    #[test]
    fn archive_rejects_unlisted_build_script_binary_and_example() {
        let build_script = Fixture::new("[package]\nname = \"example\"\nbuild = false\n", &[]);
        write_file(
            &build_script.source.join("packages/example/build.rs"),
            b"fn main() {}\n",
        );
        let error = build_script
            .verify()
            .expect_err("unlisted build script must fail");
        assert!(error.contains("unlisted Cargo build input"), "{error}");

        let binary = Fixture::new("[package]\nname = \"example\"\nbuild = false\n", &[]);
        write_file(
            &binary.source.join("packages/example/src/bin/escape.rs"),
            b"fn main() {}\n",
        );
        let error = binary.verify().expect_err("unlisted binary must fail");
        assert!(error.contains("unlisted Cargo build input"), "{error}");

        let example = Fixture::new("[package]\nname = \"example\"\nbuild = false\n", &[]);
        write_file(
            &example.source.join("packages/example/examples/escape.rs"),
            b"fn main() {}\n",
        );
        let error = example.verify().expect_err("unlisted example must fail");
        assert!(error.contains("unlisted Cargo build input"), "{error}");
    }

    #[test]
    fn archive_accepts_absent_or_inventoried_build_script() {
        let absent = Fixture::new("[package]\nname = \"example\"\n", &[]);
        assert!(absent.verify().is_ok());

        let present = Fixture::new(
            "[package]\nname = \"example\"\n",
            &[("packages/example/build.rs", "fn main() {}\n")],
        );
        assert!(present.verify().is_ok());
    }

    #[test]
    fn archive_covers_proc_macro_and_local_build_dependency_inputs() {
        let fixture = Fixture::with_packages(
            &[
                (
                    "packages/example/Cargo.toml",
                    "[package]\nname = \"example\"\n\
                     [build-dependencies]\nhelper = { path = \"../helper\" }\n",
                ),
                ("packages/example/src/lib.rs", "pub fn example() {}\n"),
                ("packages/example/build.rs", "fn main() {}\n"),
                (
                    "packages/helper/Cargo.toml",
                    "[package]\nname = \"helper\"\nbuild = false\n\
                     [lib]\nproc-macro = true\n",
                ),
                ("packages/helper/src/lib.rs", "extern crate proc_macro;\n"),
            ],
            &[],
        );
        assert!(fixture.verify().is_ok());

        let unpinned = Fixture::new(
            "[package]\nname = \"example\"\n\
             [build-dependencies]\nhelper = { path = \"../helper\" }\n",
            &[("packages/example/build.rs", "fn main() {}\n")],
        );
        write_file(
            &unpinned.source.join("packages/helper/Cargo.toml"),
            b"[package]\nname = \"helper\"\nbuild = false\n",
        );
        write_file(
            &unpinned.source.join("packages/helper/src/lib.rs"),
            b"pub fn helper() {}\n",
        );
        let error = unpinned
            .verify()
            .expect_err("unpinned build dependency must fail");
        assert!(
            error.contains("unpinned canonical build dependency"),
            "{error}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn archive_rejects_symlinked_and_traversing_inputs() {
        use std::os::unix::fs::symlink;

        let symlinked = Fixture::new("[package]\nname = \"example\"\nbuild = false\n", &[]);
        let source_file = symlinked.source.join("packages/example/src/lib.rs");
        let outside = symlinked.root.join("outside.rs");
        fs::write(&outside, b"pub fn example() {}\n").expect("write symlink target");
        fs::remove_file(&source_file).expect("remove source file");
        symlink(&outside, &source_file).expect("create symlink");
        let error = symlinked.verify().expect_err("symlinked input must fail");
        assert!(error.contains("symbolic link"), "{error}");

        let traversal = Fixture::new(
            "[package]\nname = \"example\"\nbuild = false\n\
             [[bin]]\nname = \"escape\"\npath = \"../../../escape.rs\"\n",
            &[],
        );
        let error = traversal.verify().expect_err("traversing input must fail");
        assert!(error.contains("unsafe Cargo path"), "{error}");
    }
}
