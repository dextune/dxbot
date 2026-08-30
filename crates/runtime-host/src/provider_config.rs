//! Owner-only production Provider composition.
//!
//! Runtime Host owns config loading/composition; ProviderHost remains the
//! registration/lifecycle owner. Config stores only a credential reference.
//! Secret material is resolved ephemerally and is never serialized or logged.
//!
//! Adapter selection is resolved through the static `ProviderFactory` inventory
//! (`DXB-DEL-068` H5); there is no concrete adapter `match` here. An unknown
//! adapter key or a schema mismatch is a typed fail-closed error.

#![forbid(unsafe_code)]

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

#[cfg(feature = "dsh-acp")]
use provider_host::{AcpFactoryConfig, AcpLimits, PermissionPolicy};
use provider_host::{
    FactoryConfig, FactoryError, ProviderHost, ProviderStatus, factory_keys, lookup_factory,
};
use serde::Deserialize;
#[cfg(feature = "dsh-acp")]
use sha2::{Digest, Sha256};

pub(crate) const PROVIDER_CONFIG_FILE: &str = "provider-config.json";
const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MIN_TIMEOUT_MS: u64 = 100;
const MAX_TIMEOUT_MS: u64 = 120_000;

/// The compile-time audited bytes of the current repository DSH ACP profile
/// patch. The audited digest is computed from *these* embedded bytes at build
/// time, so it always reflects the actual `deploy/dsh/dsh-acp.patch.yaml` that
/// ships in this binary — never a value copied from any prompt or note. A
/// deployment's on-disk patch must hash to exactly this digest. Present only
/// with the `dsh-acp` slice.
#[cfg(feature = "dsh-acp")]
const AUDITED_PATCH_BYTES: &[u8] = include_bytes!("../../../deploy/dsh/dsh-acp.patch.yaml");

/// Maximum bytes for a validated command executable or audited patch file. The
/// child executable and patch are both bounded so a runaway file cannot be
/// hashed unboundedly.
#[cfg(feature = "dsh-acp")]
const MAX_COMMAND_BYTES: u64 = 512 * 1024 * 1024;
#[cfg(feature = "dsh-acp")]
const MAX_PATCH_BYTES: u64 = 1024 * 1024;

#[derive(Debug)]
pub(crate) struct ProviderComposition {
    pub host: ProviderHost,
    pub provider_id: String,
    pub provider_generation: i64,
    pub provider_ready: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderConfig {
    schema_version: u32,
    adapter: String,
    provider_id: String,
    capability: String,
    generation: i64,
    #[serde(default)]
    endpoint: String,
    model: String,
    credential_ref: String,
    timeout_ms: u64,
    /// Schema v2 harness block, required only for the DSH ACP adapter. Absent
    /// for schema v1 direct configs, which are unchanged. Present only with the
    /// `dsh-acp` slice; when that slice is removed a harness config is rejected
    /// as an unknown adapter (fail closed), and this field is not decoded.
    #[cfg(feature = "dsh-acp")]
    #[serde(default)]
    harness: Option<HarnessConfig>,
}

/// Schema v2 harness block for the official DeepSeek Harness ACP adapter.
#[cfg(feature = "dsh-acp")]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HarnessConfig {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    patch_path: String,
    dsh_home: String,
    workspace: String,
    path_env: String,
    #[serde(default)]
    permission: HarnessPermission,
    limits: HarnessLimits,
    /// Pinned upstream source commit; must equal the pinned identity.
    source_commit: String,
    /// Pinned upstream source version; must equal the pinned identity.
    source_version: String,
    /// Expected lowercase 64-hex SHA-256 of the on-disk command executable.
    command_sha256: String,
    /// Expected lowercase 64-hex SHA-256 of the on-disk audited patch file.
    patch_sha256: String,
    /// Optional deployment resource-governance policy (`DXB-DEL-068` H10
    /// Task 9). Additive and optional: absent means only the process-model
    /// bounds (single ACP process/call, queue/frame/output/stderr/deadline)
    /// apply. Present, it declares how each OS resource limit is enforced. A
    /// limit declared as required-in-process fails composition closed, because
    /// DXBOT cannot prove unprivileged in-process OS limit enforcement under
    /// the crate's forbid-unsafe policy; such limits must be delegated to an
    /// explicit deployment mechanism (systemd/DSH sandbox) or be absent.
    #[serde(default)]
    resource_policy: Option<HarnessResourcePolicy>,
}

/// Declared enforcement for one OS resource limit. The runtime never pretends
/// an unprivileged in-process control exists; enforcement is either delegated
/// to an explicit, named deployment mechanism (defense in depth, never
/// canonical authority) or the composition fails closed.
#[cfg(feature = "dsh-acp")]
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum ResourceEnforcement {
    /// The limit is enforced by a named deployment mechanism (e.g. a systemd
    /// unit's `CPUQuota`/`MemoryMax`/`TasksMax`/`LimitNOFILE`, or a DSH sandbox
    /// policy). The runtime records the reference but does not itself apply the
    /// OS limit. This is the safe, honest mode: enforcement is real but owned
    /// by the deployment, and the runtime does not fake it.
    DeploymentPolicy,
    /// The limit is required to be enforced in-process by the runtime. DXBOT
    /// cannot prove this without privileged/unsafe controls, so a composition
    /// requesting it FAILS CLOSED rather than running unrestricted.
    RequiredInProcess,
}

/// One declared OS resource limit and how it is enforced.
#[cfg(feature = "dsh-acp")]
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ResourceLimitPolicy {
    /// Positive-finite maximum (units are resource-specific; validated > 0).
    max: u64,
    enforcement: ResourceEnforcement,
    /// Bounded, opaque reference to the deployment mechanism that enforces this
    /// limit (e.g. `systemd:MemoryMax` or `dsh-sandbox:mem`). Required when the
    /// enforcement is `deployment-policy`; ignored otherwise.
    #[serde(default)]
    deployment_ref: String,
}

/// Deployment resource-governance policy for the ACP child. Every present
/// limit has a positive-finite maximum and a declared enforcement; none may be
/// silently unrestricted.
#[cfg(feature = "dsh-acp")]
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct HarnessResourcePolicy {
    /// Max concurrent DSH processes per this provider (positive; P0 fixes 1).
    max_concurrent_processes: u32,
    #[serde(default)]
    cpu: Option<ResourceLimitPolicy>,
    #[serde(default)]
    memory: Option<ResourceLimitPolicy>,
    #[serde(default)]
    pids: Option<ResourceLimitPolicy>,
    #[serde(default)]
    open_files: Option<ResourceLimitPolicy>,
    #[serde(default)]
    file_size: Option<ResourceLimitPolicy>,
    /// Network policy reference (bounded, opaque). Empty means the default
    /// no-network deployment stance; a non-empty value names the deployment
    /// mechanism that enforces it. Never an in-process fabrication.
    #[serde(default)]
    network_ref: String,
}

#[cfg(feature = "dsh-acp")]
#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum HarnessPermission {
    #[default]
    Reject,
    AllowOnce,
}

#[cfg(feature = "dsh-acp")]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HarnessLimits {
    max_line_bytes: usize,
    max_output_bytes: usize,
    max_update_frames: usize,
    max_stderr_bytes: usize,
    handshake_deadline_ms: u64,
    prompt_deadline_ms: u64,
    eof_grace_ms: u64,
    term_grace_ms: u64,
}

pub(crate) fn load_provider_composition(
    runtime_root: &Path,
    handle: tokio::runtime::Handle,
) -> Result<ProviderComposition, io::Error> {
    load_with_secret_resolver(runtime_root, handle, |name| std::env::var(name).ok())
}

fn load_with_secret_resolver(
    runtime_root: &Path,
    handle: tokio::runtime::Handle,
    resolve_secret: impl FnOnce(&str) -> Option<String>,
) -> Result<ProviderComposition, io::Error> {
    let path = runtime_root.join(PROVIDER_CONFIG_FILE);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ProviderComposition {
                host: ProviderHost::new().with_runtime_handle(handle),
                provider_id: "unconfigured".to_owned(),
                provider_generation: 0,
                provider_ready: false,
            });
        }
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(invalid_config(
            "Provider config must be a direct regular file",
        ));
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Provider config must be owner-only (mode 0600)",
        ));
    }
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(invalid_config("Provider config exceeds 64 KiB"));
    }
    let bytes = fs::read(&path)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_CONFIG_BYTES {
        return Err(invalid_config("Provider config grew beyond 64 KiB"));
    }
    let config: ProviderConfig = serde_json::from_slice(&bytes)
        .map_err(|_| invalid_config("Provider config is not valid schema v1 JSON"))?;
    validate_config(&config)?;

    // Resolve the factory from the static inventory. Unknown keys fail closed.
    let factory = lookup_factory(&config.adapter).map_err(map_factory_error)?;
    if factory.schema_version != config.schema_version {
        return Err(invalid_config(
            "Provider config schema version is unsupported by the selected factory",
        ));
    }

    // For the DSH ACP adapter, validate the on-disk command and patch, the
    // isolated directories, and every digest BEFORE any secret is resolved.
    // This fails closed with path/digest/content-free diagnostics and never
    // touches the credential unless the deployment layout is already trusted.
    #[cfg(feature = "dsh-acp")]
    if let Some(harness) = config.harness.as_ref() {
        validate_harness_preflight(harness)?;
    }

    let secret_name = config
        .credential_ref
        .strip_prefix("env:")
        .ok_or_else(|| invalid_config("credential_ref must use env:<NAME>"))?;
    if !valid_secret_name(secret_name) {
        return Err(invalid_config(
            "credential_ref contains an invalid environment name",
        ));
    }
    let secret = resolve_secret(secret_name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Provider credential reference is unresolved: env:{secret_name}"),
        )
    })?;
    if secret.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Provider credential reference is empty: env:{secret_name}"),
        ));
    }

    #[cfg(feature = "dsh-acp")]
    let acp = config.harness.as_ref().map(|harness| AcpFactoryConfig {
        command: harness.command.clone(),
        args: harness.args.clone(),
        patch_path: harness.patch_path.clone(),
        dsh_home: harness.dsh_home.clone(),
        workspace: harness.workspace.clone(),
        path_env: harness.path_env.clone(),
        permission: match harness.permission {
            HarnessPermission::Reject => PermissionPolicy::Reject,
            HarnessPermission::AllowOnce => PermissionPolicy::AllowOnce,
        },
        limits: AcpLimits {
            max_line_bytes: harness.limits.max_line_bytes,
            max_output_bytes: harness.limits.max_output_bytes,
            max_update_frames: harness.limits.max_update_frames,
            max_stderr_bytes: harness.limits.max_stderr_bytes,
            handshake_deadline: Duration::from_millis(harness.limits.handshake_deadline_ms),
            prompt_deadline: Duration::from_millis(harness.limits.prompt_deadline_ms),
            eof_grace: Duration::from_millis(harness.limits.eof_grace_ms),
            term_grace: Duration::from_millis(harness.limits.term_grace_ms),
        },
        source_commit: harness.source_commit.clone(),
        source_version: harness.source_version.clone(),
        command_sha256: harness.command_sha256.clone(),
        patch_sha256: harness.patch_sha256.clone(),
        // Thread the single canonical audited digest down to the provider so
        // the spawn-time recheck compares against the same value composition
        // used. There is no second audited digest computed inside the provider.
        audited_patch_sha256: audited_patch_digest(),
    });
    let factory_config = FactoryConfig {
        provider_id: &config.provider_id,
        capability: &config.capability,
        generation: config.generation,
        endpoint: &config.endpoint,
        model: &config.model,
        timeout: Duration::from_millis(config.timeout_ms),
        secret: &secret,
        #[cfg(feature = "dsh-acp")]
        acp,
    };
    let registration = (factory.build)(&factory_config).map_err(map_factory_error)?;
    drop(secret);

    let host = ProviderHost::new().with_runtime_handle(handle);
    host.register(registration)
        .map_err(|_| invalid_config("Provider registration is invalid"))?;

    let registered = host
        .list_providers(Some(&config.capability))
        .into_iter()
        .find(|provider| provider.id.0 == config.provider_id)
        .ok_or_else(|| invalid_config("Provider registration was not retained"))?;

    Ok(ProviderComposition {
        host,
        provider_id: registered.id.0,
        provider_generation: registered.generation,
        provider_ready: registered.status == ProviderStatus::Ready,
    })
}

/// Validate the DSH ACP command and patch on disk, the isolated directories,
/// and every configured digest, before any secret is resolved.
///
/// All diagnostics are category-only: they never include a path, a digest, or
/// file content, so a leaked error cannot expose the deployment layout, the
/// audited composition, or an executable fingerprint.
#[cfg(feature = "dsh-acp")]
fn validate_harness_preflight(harness: &HarnessConfig) -> Result<(), io::Error> {
    // The pinned upstream identity must match exactly. There is no floating
    // branch or `latest` follow; a mismatch fails closed.
    if harness.source_commit != provider_host::ACP_SOURCE_COMMIT {
        return Err(invalid_config(
            "Harness source commit does not match the pinned identity",
        ));
    }
    if harness.source_version != provider_host::ACP_SOURCE_VERSION {
        return Err(invalid_config(
            "Harness source version does not match the pinned identity",
        ));
    }
    // Both configured digests must be well-formed lowercase 64-hex SHA-256.
    if !is_lowercase_sha256_hex(&harness.command_sha256)
        || !is_lowercase_sha256_hex(&harness.patch_sha256)
    {
        return Err(invalid_config(
            "Harness digest is not a lowercase 64-hex SHA-256",
        ));
    }

    // The argv must be exactly `--profile acp --patch <patch_path>`, with the
    // final element equal to the configured patch path. No omission, extra,
    // duplicate, reorder, or mismatch is tolerated. This mirrors the
    // provider-side invariant but fails closed here, before any secret is
    // resolved.
    if !argv_is_exact(&harness.args, &harness.patch_path) {
        return Err(invalid_config(
            "Harness argv must be exactly --profile acp --patch <patch_path>",
        ));
    }

    // The command must be a direct regular file (no symlink), executable by the
    // owner, neither group- nor other-writable, and bounded in size. Its digest
    // must equal the deployment-specific configured command digest.
    let command_digest = validate_regular_file(
        Path::new(&harness.command),
        MAX_COMMAND_BYTES,
        FileRule::Executable,
        "Harness command",
    )?;
    if !digests_equal(&command_digest, &harness.command_sha256) {
        return Err(invalid_config(
            "Harness command digest does not match the configured value",
        ));
    }

    // The patch must be a direct regular file (no symlink), not group/other
    // writable, and bounded. Its digest must equal BOTH the configured value
    // and the compile-time audited digest of the current repository patch.
    let patch_digest = validate_regular_file(
        Path::new(&harness.patch_path),
        MAX_PATCH_BYTES,
        FileRule::Data,
        "Harness patch",
    )?;
    if !digests_equal(&patch_digest, &harness.patch_sha256) {
        return Err(invalid_config(
            "Harness patch digest does not match the configured value",
        ));
    }
    let audited = audited_patch_digest();
    if !digests_equal(&patch_digest, &audited) {
        return Err(invalid_config(
            "Harness patch digest does not match the audited composition",
        ));
    }

    // DSH_HOME and the workspace must be direct existing directories (no
    // symlink). DSH_HOME must additionally be owner-only so the isolated home
    // cannot be read or written by group/other.
    validate_directory(Path::new(&harness.dsh_home), true, "Harness DSH_HOME")?;
    validate_directory(Path::new(&harness.workspace), false, "Harness workspace")?;

    // Deployment resource-governance policy (`DXB-DEL-068` H10 Task 9). Every
    // declared limit must be positive-finite; a limit declared as required
    // in-process fails closed because unprivileged in-process OS enforcement
    // cannot be proven under the crate's forbid-unsafe policy.
    if let Some(policy) = harness.resource_policy.as_ref() {
        validate_resource_policy(policy)?;
    }
    Ok(())
}

/// Validate the deployment resource-governance policy, failing closed on a
/// non-positive maximum, a missing deployment reference for a delegated limit,
/// or any limit that demands unprovable in-process enforcement.
#[cfg(feature = "dsh-acp")]
fn validate_resource_policy(policy: &HarnessResourcePolicy) -> Result<(), io::Error> {
    if policy.max_concurrent_processes == 0 {
        return Err(invalid_config(
            "Harness resource policy max concurrent processes must be positive",
        ));
    }
    for (label, limit) in [
        ("cpu", policy.cpu.as_ref()),
        ("memory", policy.memory.as_ref()),
        ("pids", policy.pids.as_ref()),
        ("open-files", policy.open_files.as_ref()),
        ("file-size", policy.file_size.as_ref()),
    ] {
        let Some(limit) = limit else {
            continue;
        };
        if limit.max == 0 {
            return Err(invalid_config(&format!(
                "Harness resource limit '{label}' maximum must be positive-finite"
            )));
        }
        match limit.enforcement {
            ResourceEnforcement::DeploymentPolicy => {
                // A delegated limit must name the mechanism that enforces it,
                // so operators can verify the deployment actually applies it.
                if limit.deployment_ref.trim().is_empty() || limit.deployment_ref.len() > 256 {
                    return Err(invalid_config(&format!(
                        "Harness resource limit '{label}' delegates enforcement but names no deployment mechanism"
                    )));
                }
            }
            ResourceEnforcement::RequiredInProcess => {
                // Fail closed: DXBOT cannot prove unprivileged in-process OS
                // limit enforcement under forbid-unsafe. Rather than run
                // unrestricted while pretending the control exists, refuse the
                // composition. The operator must either delegate the limit to a
                // deployment mechanism or drop the requirement.
                return Err(invalid_config(&format!(
                    "Harness resource limit '{label}' requires in-process enforcement that cannot be proven; delegate it to a deployment mechanism or remove it"
                )));
            }
        }
    }
    if policy.network_ref.len() > 256 {
        return Err(invalid_config(
            "Harness network policy reference exceeds its bounded length",
        ));
    }
    Ok(())
}

/// Whether a validated file must additionally be owner-executable.
#[cfg(feature = "dsh-acp")]
#[derive(Clone, Copy)]
enum FileRule {
    Executable,
    Data,
}

/// Validate a direct regular file (rejecting symlinks), enforce that it is
/// neither group- nor other-writable, optionally require owner-execute, bound
/// its size, and return its lowercase 64-hex SHA-256. Errors are category-only
/// and never include the path, size, or digest.
#[cfg(feature = "dsh-acp")]
fn validate_regular_file(
    path: &Path,
    max_bytes: u64,
    rule: FileRule,
    label: &str,
) -> Result<String, io::Error> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| invalid_config(&format!("{label} is not accessible")))?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || !file_type.is_file() {
        return Err(invalid_config(&format!(
            "{label} must be a direct regular file"
        )));
    }
    let mode = metadata.permissions().mode();
    // Reject any group- or other-writable file (0o022 write bits).
    if mode & 0o022 != 0 {
        return Err(invalid_config(&format!(
            "{label} must not be group- or other-writable"
        )));
    }
    if let FileRule::Executable = rule {
        // Require owner-execute so the command is actually runnable.
        if mode & 0o100 == 0 {
            return Err(invalid_config(&format!("{label} must be owner-executable")));
        }
    }
    if metadata.len() > max_bytes {
        return Err(invalid_config(&format!("{label} exceeds its bounded size")));
    }
    let bytes = fs::read(path).map_err(|_| invalid_config(&format!("{label} is not readable")))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > max_bytes {
        return Err(invalid_config(&format!(
            "{label} grew beyond its bounded size"
        )));
    }
    Ok(sha256_hex(&bytes))
}

/// Validate that a path is a direct existing directory (rejecting symlinks),
/// optionally requiring it be owner-only (no group/other access at all).
#[cfg(feature = "dsh-acp")]
fn validate_directory(path: &Path, owner_only: bool, label: &str) -> Result<(), io::Error> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| invalid_config(&format!("{label} is not accessible")))?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || !file_type.is_dir() {
        return Err(invalid_config(&format!(
            "{label} must be a direct existing directory"
        )));
    }
    if owner_only && metadata.permissions().mode() & 0o077 != 0 {
        return Err(invalid_config(&format!("{label} must be owner-only")));
    }
    Ok(())
}

/// Compute the compile-time audited patch digest from the embedded repository
/// bytes. Computed fresh from `include_bytes!` so it always reflects the actual
/// shipped `deploy/dsh/dsh-acp.patch.yaml`.
#[cfg(feature = "dsh-acp")]
fn audited_patch_digest() -> String {
    sha256_hex(AUDITED_PATCH_BYTES)
}

#[cfg(feature = "dsh-acp")]
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Length-checked byte-diff equality that does not early-return on the first
/// differing byte. Both inputs are already lowercase 64-hex.
#[cfg(feature = "dsh-acp")]
fn digests_equal(actual: &str, expected: &str) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (a, b) in actual.bytes().zip(expected.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(feature = "dsh-acp")]
fn is_lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The exact pinned argv: `--profile acp --patch <patch_path>`, where the final
/// element equals the configured patch path.
#[cfg(feature = "dsh-acp")]
fn argv_is_exact(args: &[String], patch_path: &str) -> bool {
    args.len() == 4
        && args[0] == provider_host::ACP_ARG_PROFILE_FLAG
        && args[1] == provider_host::ACP_ARG_PROFILE_VALUE
        && args[2] == provider_host::ACP_ARG_PATCH_FLAG
        && args[3] == patch_path
}

fn validate_config(config: &ProviderConfig) -> Result<(), io::Error> {
    // The adapter must be a known factory key; concrete adapter names are not
    // hard-coded here, they are derived from the static inventory.
    if !factory_keys().contains(&config.adapter.as_str()) {
        return Err(invalid_config("unsupported production Provider adapter"));
    }
    // Whether this is the DSH ACP adapter. With the `dsh-acp` slice removed the
    // key is not in `factory_keys()` (rejected above), so this is always false
    // and no ACP-specific config path is compiled in.
    #[cfg(feature = "dsh-acp")]
    let is_acp = config.adapter == provider_host::ACP_ADAPTER_KEY;
    #[cfg(not(feature = "dsh-acp"))]
    let is_acp = false;
    // Schema v1 = HTTP direct adapters (unchanged); schema v2 = DSH ACP.
    if is_acp {
        if config.schema_version != 2 {
            return Err(invalid_config(
                "deepseek-harness-acp requires Provider config schema version 2",
            ));
        }
    } else if config.schema_version != 1 {
        return Err(invalid_config("unsupported Provider config schema version"));
    }
    for (name, value) in [
        ("provider_id", config.provider_id.as_str()),
        ("capability", config.capability.as_str()),
        ("model", config.model.as_str()),
    ] {
        if value.trim().is_empty() || value.len() > 256 {
            return Err(invalid_config(&format!(
                "invalid Provider config field: {name}"
            )));
        }
    }
    if config.generation <= 0 {
        return Err(invalid_config("Provider generation must be positive"));
    }
    if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&config.timeout_ms) {
        return Err(invalid_config(
            "Provider timeout_ms is outside the bounded range",
        ));
    }
    #[cfg(feature = "dsh-acp")]
    if is_acp {
        // DSH ACP requires the harness block and forbids an HTTP endpoint.
        if config.harness.is_none() {
            return Err(invalid_config(
                "deepseek-harness-acp requires a harness config block",
            ));
        }
        if !config.endpoint.is_empty() {
            return Err(invalid_config(
                "deepseek-harness-acp must not carry an HTTP endpoint",
            ));
        }
        // The model must be the exact pinned route model; no other model.
        if config.model != provider_host::ACP_ROUTE_MODEL {
            return Err(invalid_config(
                "deepseek-harness-acp model must be exactly MiniMax-M3",
            ));
        }
        return Ok(());
    }
    // HTTP direct adapters: harness must be absent, endpoint must be bounded.
    #[cfg(feature = "dsh-acp")]
    if config.harness.is_some() {
        return Err(invalid_config(
            "schema v1 direct adapter must not carry a harness block",
        ));
    }
    if config.endpoint.len() > 2048
        || !(config.endpoint.starts_with("http://") || config.endpoint.starts_with("https://"))
        || config
            .endpoint
            .split_once("://")
            .is_some_and(|(_, authority)| authority.contains('@'))
    {
        return Err(invalid_config(
            "Provider endpoint must be bounded HTTP(S) without embedded credentials",
        ));
    }
    Ok(())
}

fn valid_secret_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_uppercase() || (index > 0 && byte.is_ascii_digit())
        })
}

fn map_factory_error(error: FactoryError) -> io::Error {
    match error {
        FactoryError::UnknownAdapter { .. } => {
            invalid_config("unsupported production Provider adapter")
        }
        FactoryError::CredentialUnavailable => {
            invalid_config("Provider credential cannot be installed")
        }
        FactoryError::InvalidConfig { .. } => invalid_config("Provider registration is invalid"),
    }
}

fn invalid_config(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempRoot(std::path::PathBuf);

    impl TempRoot {
        fn new() -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("dxb-provider-config-{suffix}"));
            fs::create_dir_all(&path).expect("create temp root");
            Self(path)
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn test_handle() -> (tokio::runtime::Runtime, tokio::runtime::Handle) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let handle = runtime.handle().clone();
        (runtime, handle)
    }

    fn write_config(root: &Path, credential_ref: &str) {
        let path = root.join(PROVIDER_CONFIG_FILE);
        fs::write(
            &path,
            serde_json::json!({
                "schema_version": 1,
                "adapter": "deepseek-flash",
                "provider_id": "deepseek-production",
                "capability": "llm-chat",
                "generation": 7,
                "endpoint": "http://127.0.0.1:10000",
                "model": "test-model",
                "credential_ref": credential_ref,
                "timeout_ms": 30000
            })
            .to_string(),
        )
        .expect("write config");
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("permissions");
    }

    fn write_minimax_config(root: &Path, credential_ref: &str) {
        let path = root.join(PROVIDER_CONFIG_FILE);
        fs::write(
            &path,
            serde_json::json!({
                "schema_version": 1,
                "adapter": "minimax-m3",
                "provider_id": "minimax-m3-production",
                "capability": "llm-chat",
                "generation": 3,
                "endpoint": "https://api.minimax.io/anthropic/v1",
                "model": "MiniMax-M3",
                "credential_ref": credential_ref,
                "timeout_ms": 30000
            })
            .to_string(),
        )
        .expect("write config");
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("permissions");
    }

    #[test]
    fn minimax_anthropic_config_registers_ready_without_secret_projection() {
        let root = TempRoot::new();
        write_minimax_config(&root.0, "env:MINIMAX_API_KEY");
        let secret = "minimax-secret-canary-must-not-appear";
        let (_runtime, handle) = test_handle();
        let composition = load_with_secret_resolver(&root.0, handle, |name| {
            assert_eq!(name, "MINIMAX_API_KEY");
            Some(secret.to_owned())
        })
        .expect("configured MiniMax");
        assert_eq!(composition.provider_id, "minimax-m3-production");
        assert_eq!(composition.provider_generation, 3);
        assert!(composition.provider_ready);
        assert!(!format!("{composition:?}").contains(secret));
    }

    #[test]
    fn absent_config_is_explicitly_unconfigured_without_reference_fallback() {
        let root = TempRoot::new();
        let (_runtime, handle) = test_handle();
        let composition =
            load_with_secret_resolver(&root.0, handle, |_| None).expect("unconfigured");
        assert_eq!(composition.provider_id, "unconfigured");
        assert!(!composition.provider_ready);
        assert!(composition.host.list_providers(None).is_empty());
    }

    #[test]
    fn configured_provider_uses_reference_not_secret_in_projection() {
        let root = TempRoot::new();
        write_config(&root.0, "env:DXBOT_TEST_PROVIDER_TOKEN");
        let secret = "secret-canary-must-not-appear";
        let (_runtime, handle) = test_handle();
        let composition = load_with_secret_resolver(&root.0, handle, |name| {
            assert_eq!(name, "DXBOT_TEST_PROVIDER_TOKEN");
            Some(secret.to_owned())
        })
        .expect("configured");
        assert_eq!(composition.provider_id, "deepseek-production");
        assert!(composition.provider_ready);
        let rendered = format!("{composition:?}");
        assert!(!rendered.contains(secret));
        let providers = composition.host.list_providers(None);
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].generation, 7);
    }

    #[test]
    fn unresolved_credential_fails_closed_without_secret_material() {
        let root = TempRoot::new();
        write_config(&root.0, "env:DXBOT_MISSING_PROVIDER_TOKEN");
        let (_runtime, handle) = test_handle();
        let error = load_with_secret_resolver(&root.0, handle, |_| None).expect_err("must fail");
        let rendered = error.to_string();
        assert!(rendered.contains("env:DXBOT_MISSING_PROVIDER_TOKEN"));
        assert!(!rendered.contains("Bearer"));
    }

    #[test]
    fn unknown_adapter_key_fails_closed() {
        let root = TempRoot::new();
        let path = root.0.join(PROVIDER_CONFIG_FILE);
        fs::write(
            &path,
            serde_json::json!({
                "schema_version": 1,
                "adapter": "no-such-adapter",
                "provider_id": "x",
                "capability": "llm-chat",
                "generation": 1,
                "endpoint": "https://example.test",
                "model": "m",
                "credential_ref": "env:DXBOT_TEST_PROVIDER_TOKEN",
                "timeout_ms": 30000
            })
            .to_string(),
        )
        .expect("write config");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("permissions");
        let (_runtime, handle) = test_handle();
        let error = load_with_secret_resolver(&root.0, handle, |_| Some("s".to_owned()))
            .expect_err("unknown adapter must fail closed");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn config_permissions_are_owner_only() {
        let root = TempRoot::new();
        write_config(&root.0, "env:DXBOT_TEST_PROVIDER_TOKEN");
        fs::set_permissions(
            root.0.join(PROVIDER_CONFIG_FILE),
            fs::Permissions::from_mode(0o644),
        )
        .expect("permissions");
        let (_runtime, handle) = test_handle();
        let error = load_with_secret_resolver(&root.0, handle, |_| Some("secret".to_owned()))
            .expect_err("must reject broad permissions");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }

    /// DSH ACP config-decoding tests. Present only with the `dsh-acp` slice;
    /// removing the slice removes ACP config decoding and these tests.
    #[cfg(feature = "dsh-acp")]
    mod acp {
        use super::super::*;
        use super::{TempRoot, test_handle};
        use std::os::unix::fs::PermissionsExt;

        /// A fully-valid on-disk DSH ACP deployment layout inside `root`:
        /// an owner-executable command, the audited patch (byte-identical to the
        /// shipped repo patch so its digest matches the compile-time audited
        /// digest), an owner-only DSH_HOME, and a workspace directory. Returns the
        /// harness JSON with correct paths and digests.
        struct AcpLayout {
            command: std::path::PathBuf,
            patch: std::path::PathBuf,
            dsh_home: std::path::PathBuf,
            workspace: std::path::PathBuf,
            command_sha256: String,
            patch_sha256: String,
        }

        impl AcpLayout {
            fn new(root: &Path) -> Self {
                let command = root.join("dsh");
                fs::write(&command, b"#!/bin/sh\nexit 0\n").expect("write command");
                fs::set_permissions(&command, fs::Permissions::from_mode(0o700))
                    .expect("command mode");
                let patch = root.join("dsh-acp.patch.yaml");
                fs::write(&patch, AUDITED_PATCH_BYTES).expect("write patch");
                fs::set_permissions(&patch, fs::Permissions::from_mode(0o600)).expect("patch mode");
                let dsh_home = root.join("dsh-home");
                fs::create_dir_all(&dsh_home).expect("dsh_home");
                fs::set_permissions(&dsh_home, fs::Permissions::from_mode(0o700))
                    .expect("dsh_home mode");
                let workspace = root.join("workspace");
                fs::create_dir_all(&workspace).expect("workspace");
                let command_sha256 = sha256_hex(&fs::read(&command).expect("read command"));
                let patch_sha256 = sha256_hex(AUDITED_PATCH_BYTES);
                Self {
                    command,
                    patch,
                    dsh_home,
                    workspace,
                    command_sha256,
                    patch_sha256,
                }
            }

            fn harness_json(&self) -> serde_json::Value {
                let patch = self.patch.to_str().unwrap().to_owned();
                serde_json::json!({
                    "command": self.command.to_str().unwrap(),
                    "args": ["--profile", "acp", "--patch", patch],
                    "patch_path": patch,
                    "dsh_home": self.dsh_home.to_str().unwrap(),
                    "workspace": self.workspace.to_str().unwrap(),
                    "path_env": "/usr/bin:/bin",
                    "permission": "reject",
                    "source_commit": provider_host::ACP_SOURCE_COMMIT,
                    "source_version": provider_host::ACP_SOURCE_VERSION,
                    "command_sha256": self.command_sha256,
                    "patch_sha256": self.patch_sha256,
                    "limits": {
                        "max_line_bytes": 65536,
                        "max_output_bytes": 262144,
                        "max_update_frames": 1024,
                        "max_stderr_bytes": 8192,
                        "handshake_deadline_ms": 10000,
                        "prompt_deadline_ms": 30000,
                        "eof_grace_ms": 6000,
                        "term_grace_ms": 3000
                    }
                })
            }
        }

        fn acp_config_value(layout: &AcpLayout) -> serde_json::Value {
            serde_json::json!({
                "schema_version": 2,
                "adapter": "deepseek-harness-acp",
                "provider_id": "deepseek-harness-acp",
                "capability": "llm-chat",
                "generation": 1,
                "model": "MiniMax-M3",
                "credential_ref": "env:MINIMAX_API_KEY",
                "timeout_ms": 60000,
                "harness": layout.harness_json()
            })
        }

        fn write_value(root: &Path, value: &serde_json::Value) {
            let path = root.join(PROVIDER_CONFIG_FILE);
            fs::write(&path, value.to_string()).expect("write acp config");
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("permissions");
        }

        fn expect_acp_error(root: &Path, value: &serde_json::Value) -> io::Error {
            write_value(root, value);
            let (_runtime, handle) = test_handle();
            let error = load_with_secret_resolver(root, handle, |_| {
                panic!("secret must NOT be resolved when harness preflight fails")
            })
            .expect_err("acp preflight must fail closed");
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            // Category-only diagnostics: never leak a path, digest, or content.
            let rendered = error.to_string();
            assert!(
                !rendered.contains(root.to_str().unwrap()),
                "path leaked: {rendered}"
            );
            assert!(
                !rendered.contains("dsh-acp.patch.yaml"),
                "path leaked: {rendered}"
            );
            assert!(
                !rendered.contains(provider_host::ACP_SOURCE_COMMIT),
                "commit leaked: {rendered}"
            );
            error
        }

        #[test]
        fn dsh_acp_schema_v2_registers_ready_without_secret_projection() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            write_value(&root.0, &acp_config_value(&layout));
            let secret = "minimax-key-canary-must-not-appear";
            let (_runtime, handle) = test_handle();
            let composition = load_with_secret_resolver(&root.0, handle, |name| {
                assert_eq!(name, "MINIMAX_API_KEY");
                Some(secret.to_owned())
            })
            .expect("configured DSH ACP");
            assert_eq!(composition.provider_id, "deepseek-harness-acp");
            assert_eq!(composition.provider_generation, 1);
            assert!(composition.provider_ready);
            assert!(!format!("{composition:?}").contains(secret));
        }

        #[test]
        fn dsh_acp_rejects_wrong_source_commit() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let mut value = acp_config_value(&layout);
            value["harness"]["source_commit"] = serde_json::Value::String("f".repeat(40));
            expect_acp_error(&root.0, &value);
        }

        #[test]
        fn dsh_acp_rejects_wrong_source_version() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let mut value = acp_config_value(&layout);
            value["harness"]["source_version"] = serde_json::Value::String("0.9.9".to_owned());
            expect_acp_error(&root.0, &value);
        }

        #[test]
        fn dsh_acp_rejects_wrong_argv() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let mut value = acp_config_value(&layout);
            // Drop the trailing patch argument: not the exact pinned argv.
            value["harness"]["args"] = serde_json::json!(["--profile", "acp", "--patch"]);
            expect_acp_error(&root.0, &value);
        }

        #[test]
        fn dsh_acp_rejects_mismatched_command_digest() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let mut value = acp_config_value(&layout);
            value["harness"]["command_sha256"] = serde_json::Value::String("0".repeat(64));
            expect_acp_error(&root.0, &value);
        }

        #[test]
        fn dsh_acp_rejects_mismatched_patch_digest() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            // Corrupt the on-disk patch so its digest no longer matches the audited
            // composition or the configured value.
            fs::write(&layout.patch, b"tampered patch\n").expect("tamper patch");
            fs::set_permissions(&layout.patch, fs::Permissions::from_mode(0o600))
                .expect("patch mode");
            expect_acp_error(&root.0, &acp_config_value(&layout));
        }

        #[test]
        fn dsh_acp_rejects_patch_symlink() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            // Replace the patch with a symlink to a valid audited patch elsewhere.
            let real = root.0.join("real-patch.yaml");
            fs::write(&real, AUDITED_PATCH_BYTES).expect("write real patch");
            fs::set_permissions(&real, fs::Permissions::from_mode(0o600)).expect("mode");
            fs::remove_file(&layout.patch).expect("remove patch");
            std::os::unix::fs::symlink(&real, &layout.patch).expect("symlink");
            expect_acp_error(&root.0, &acp_config_value(&layout));
        }

        #[test]
        fn dsh_acp_rejects_group_writable_command() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            // Group-writable executable is rejected.
            fs::set_permissions(&layout.command, fs::Permissions::from_mode(0o770))
                .expect("group-writable");
            let mut value = acp_config_value(&layout);
            // Recompute the command digest so only the mode (not the digest) is the
            // rejection cause.
            value["harness"]["command_sha256"] =
                serde_json::Value::String(sha256_hex(&fs::read(&layout.command).expect("read")));
            expect_acp_error(&root.0, &value);
        }

        #[test]
        fn dsh_acp_rejects_non_executable_command() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            fs::set_permissions(&layout.command, fs::Permissions::from_mode(0o600))
                .expect("non-exec");
            expect_acp_error(&root.0, &acp_config_value(&layout));
        }

        #[test]
        fn dsh_acp_rejects_non_owner_only_dsh_home() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            fs::set_permissions(&layout.dsh_home, fs::Permissions::from_mode(0o755))
                .expect("dsh_home broad");
            expect_acp_error(&root.0, &acp_config_value(&layout));
        }

        #[test]
        fn dsh_acp_rejects_dsh_home_symlink() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let real = root.0.join("real-home");
            fs::create_dir_all(&real).expect("real home");
            fs::set_permissions(&real, fs::Permissions::from_mode(0o700)).expect("mode");
            fs::remove_dir(&layout.dsh_home).expect("remove dsh_home");
            std::os::unix::fs::symlink(&real, &layout.dsh_home).expect("symlink");
            expect_acp_error(&root.0, &acp_config_value(&layout));
        }

        #[test]
        fn dsh_acp_rejects_schema_v1() {
            let root = TempRoot::new();
            let path = root.0.join(PROVIDER_CONFIG_FILE);
            fs::write(
                &path,
                serde_json::json!({
                    "schema_version": 1,
                    "adapter": "deepseek-harness-acp",
                    "provider_id": "deepseek-harness-acp",
                    "capability": "llm-chat",
                    "generation": 1,
                    "model": "MiniMax-M3",
                    "credential_ref": "env:MINIMAX_API_KEY",
                    "timeout_ms": 60000
                })
                .to_string(),
            )
            .expect("write config");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("permissions");
            let (_runtime, handle) = test_handle();
            let error = load_with_secret_resolver(&root.0, handle, |_| Some("s".to_owned()))
                .expect_err("acp requires schema v2");
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn dsh_acp_rejects_non_minimax_model() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let mut value = acp_config_value(&layout);
            value["model"] = serde_json::Value::String("some-other-model".to_owned());
            write_value(&root.0, &value);
            let (_runtime, handle) = test_handle();
            let error = load_with_secret_resolver(&root.0, handle, |_| Some("s".to_owned()))
                .expect_err("acp model must be MiniMax-M3");
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        }

        #[test]
        fn schema_v1_direct_rejects_harness_block() {
            let root = TempRoot::new();
            let path = root.0.join(PROVIDER_CONFIG_FILE);
            fs::write(
            &path,
            serde_json::json!({
                "schema_version": 1,
                "adapter": "minimax-m3",
                "provider_id": "minimax-m3-production",
                "capability": "llm-chat",
                "generation": 1,
                "endpoint": "https://api.minimax.io/anthropic/v1",
                "model": "MiniMax-M3",
                "credential_ref": "env:MINIMAX_API_KEY",
                "timeout_ms": 60000,
                "harness": {
                    "command": "/usr/bin/dsh",
                    "patch_path": "/etc/dxbot/p.yaml",
                    "dsh_home": "/var/lib/dxbot/dsh-home",
                    "workspace": "/var/lib/dxbot/workspace",
                    "path_env": "/usr/bin",
                    "source_commit": "cd5ef8148158c3a752a658978873241fdf8e2bbc",
                    "source_version": "0.1.2-alpha.1",
                    "command_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                    "patch_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                    "limits": {
                        "max_line_bytes": 1, "max_output_bytes": 1, "max_update_frames": 1,
                        "max_stderr_bytes": 1, "handshake_deadline_ms": 1, "prompt_deadline_ms": 1,
                        "eof_grace_ms": 1, "term_grace_ms": 1
                    }
                }
            })
            .to_string(),
        )
        .expect("write config");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("permissions");
            let (_runtime, handle) = test_handle();
            let error = load_with_secret_resolver(&root.0, handle, |_| Some("s".to_owned()))
                .expect_err("v1 direct must not carry harness");
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        }

        fn resource_policy(enforcement: &str, with_ref: bool, max: u64) -> serde_json::Value {
            let mut cpu = serde_json::json!({ "max": max, "enforcement": enforcement });
            if with_ref {
                cpu["deployment_ref"] = serde_json::Value::String("systemd:CPUQuota".to_owned());
            }
            serde_json::json!({
                "max_concurrent_processes": 1,
                "cpu": cpu,
                "network_ref": "systemd:IPAddressDeny"
            })
        }

        #[test]
        fn dsh_acp_delegated_resource_policy_loads_ready() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let mut value = acp_config_value(&layout);
            value["harness"]["resource_policy"] = resource_policy("deployment-policy", true, 200);
            write_value(&root.0, &value);
            let (_runtime, handle) = test_handle();
            let composition = load_with_secret_resolver(&root.0, handle, |_| Some("s".to_owned()))
                .expect("delegated resource policy loads");
            assert!(composition.provider_ready);
        }

        #[test]
        fn dsh_acp_required_in_process_resource_limit_fails_closed() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let mut value = acp_config_value(&layout);
            // A limit demanding unprovable in-process OS enforcement must refuse to
            // start the provider rather than run unrestricted.
            value["harness"]["resource_policy"] =
                resource_policy("required-in-process", false, 200);
            expect_acp_error(&root.0, &value);
        }

        #[test]
        fn dsh_acp_zero_resource_maximum_fails_closed() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let mut value = acp_config_value(&layout);
            value["harness"]["resource_policy"] = resource_policy("deployment-policy", true, 0);
            expect_acp_error(&root.0, &value);
        }

        #[test]
        fn dsh_acp_delegated_limit_without_deployment_ref_fails_closed() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let mut value = acp_config_value(&layout);
            value["harness"]["resource_policy"] = resource_policy("deployment-policy", false, 200);
            expect_acp_error(&root.0, &value);
        }

        #[test]
        fn dsh_acp_zero_concurrent_processes_fails_closed() {
            let root = TempRoot::new();
            let layout = AcpLayout::new(&root.0);
            let mut value = acp_config_value(&layout);
            value["harness"]["resource_policy"] = serde_json::json!({
                "max_concurrent_processes": 0,
                "network_ref": ""
            });
            expect_acp_error(&root.0, &value);
        }

        #[test]
        fn audited_patch_digest_matches_repo_file() {
            // The compile-time audited digest is computed from the embedded repo
            // bytes; assert it is a well-formed lowercase 64-hex SHA-256 so the
            // preflight comparison is always meaningful.
            let digest = audited_patch_digest();
            assert_eq!(digest.len(), 64);
            assert!(is_lowercase_sha256_hex(&digest));
        }
    }
}
