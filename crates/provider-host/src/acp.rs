//! Official DeepSeek Harness ACP subprocess provider (`DXB-DEL-068` H6/H7).
//!
//! `DeepSeekHarnessAcpProvider` is a production [`ExecuteProvider`](crate::execute)
//! that drives the official DeepSeek Harness (DSH) automation surface: the
//! ACP v1, newline-delimited JSON-RPC 2.0 server started by the pinned
//! executable with `--profile acp`. DXBOT stays the canonical authority; DSH is
//! a pinned, out-of-process, replaceable execution implementation.
//!
//! # Boundaries
//!
//! * No shell. The child is spawned directly (`Command::new` on the validated
//!   absolute executable) with `env_clear()` and only the explicitly allowed
//!   variables (`PATH`, `DSH_HOME`, `MINIMAX_API_KEY`) materialized.
//! * `stdout` carries ACP protocol frames only. `stderr` is captured into a
//!   bounded, redacted diagnostic buffer and never mixed into the protocol.
//! * Every execution attempt uses a **fresh process and a fresh DSH session**.
//!   No pooling, no multiplex, no automatic `session/resume`.
//! * The child runs in its own process group; teardown is a bounded ladder of
//!   optional graceful `session/close` (only when advertised) → EOF (stdin
//!   close) → `SIGTERM` → `SIGKILL`, each applied to the whole process group,
//!   and always finishes with an unconditional `SIGKILL` sweep of the group so
//!   no child *or* grandchild can leak past a graceful/EOF child exit.
//! * All limits (line/frame/output/pending/stderr bytes and every deadline) are
//!   bounded and configured. Overflow, malformed frames, unknown ids, stdout
//!   contamination, crash, and hang all fail closed with safe, category-only
//!   diagnostics. No raw stderr, protocol payload, path, prompt, or secret ever
//!   escapes into an error.
//!
//! The exact ACP wire facts (method names, `protocolVersion = 1`, the model
//! config `currentValue` of `["minimax","MiniMax-M3"]`) are pinned against the
//! official DeepSeek Harness snapshot in `DXB-DEL-064`.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use dxbot_core::types::ProviderId;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::{Instant, timeout};

use crate::cancellation::{CancellationToken, UsageInfo};
use crate::execute::{
    AllowOnceGrant, ExecuteError, ExecuteFuture, ExecuteOutcome, ExecuteProvider, ExecuteRequest,
    ExecuteResult, ToolDisposition, ToolEffectRecord,
};

/// Pinned ACP protocol version advertised by the official DSH snapshot.
/// (`@agentclientprotocol/sdk` `PROTOCOL_VERSION = 1`.)
pub const ACP_PROTOCOL_VERSION: i64 = 1;

/// The pinned production model route: provider `minimax`, model `MiniMax-M3`.
pub const ACP_ROUTE_PROVIDER: &str = "minimax";
pub const ACP_ROUTE_MODEL: &str = "MiniMax-M3";

/// The static factory/config key for the DSH ACP provider.
pub const ACP_ADAPTER_KEY: &str = "deepseek-harness-acp";

/// The exact upstream source commit this provider is pinned against, recorded
/// canonically in `DXB-DEL-064`. Any config that names a different commit fails
/// closed at construction; there is no floating branch or `latest` follow.
pub const ACP_SOURCE_COMMIT: &str = "cd5ef8148158c3a752a658978873241fdf8e2bbc";

/// The exact upstream root package version this provider is pinned against
/// (`DXB-DEL-064`, tag `dsh-v0.1.2-alpha.1`). A different version fails closed.
pub const ACP_SOURCE_VERSION: &str = "0.1.2-alpha.1";

/// The exact, complete argv the pinned DSH executable must be invoked with:
/// `--profile acp --patch <patch_path>`. The final element is the validated
/// absolute patch path, filled in at construction. No omission, extra,
/// duplicate, or reordered argument is permitted.
pub const ACP_ARG_PROFILE_FLAG: &str = "--profile";
pub const ACP_ARG_PROFILE_VALUE: &str = "acp";
pub const ACP_ARG_PATCH_FLAG: &str = "--patch";

/// Environment variable names the child is permitted to receive. Every other
/// variable is dropped by `env_clear()`; nothing ambient leaks in.
const ENV_PATH: &str = "PATH";
const ENV_DSH_HOME: &str = "DSH_HOME";
const ENV_MINIMAX_API_KEY: &str = "MINIMAX_API_KEY";

/// Upper bounds for the spawn-time asset re-hash. They mirror the Runtime
/// composition bounds so a runaway file cannot be streamed unboundedly at
/// spawn; both are intentionally generous but finite.
const MAX_SPAWN_COMMAND_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SPAWN_PATCH_BYTES: u64 = 1024 * 1024;
/// Streaming hash chunk size for the spawn-time re-hash.
const SPAWN_HASH_CHUNK_BYTES: usize = 64 * 1024;

/// How the provider answers an unattended `session/request_permission`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionPolicy {
    /// Reject every permission request (the safe unattended default).
    Reject,
    /// Select the first explicit `allow_once` option, rejecting anything else.
    AllowOnce,
}

impl PermissionPolicy {
    fn as_str(self) -> &'static str {
        match self {
            PermissionPolicy::Reject => "reject",
            PermissionPolicy::AllowOnce => "allow-once",
        }
    }
}

/// Positive, finite bounds for one ACP execution. Every field has a deployment
/// maximum resolved by the config owner; there is no adapter-hidden default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcpLimits {
    /// Maximum bytes for a single newline-delimited JSON-RPC line.
    pub max_line_bytes: usize,
    /// Maximum total assistant output bytes retained for the result.
    pub max_output_bytes: usize,
    /// Maximum number of `session/update` frames consumed for one prompt.
    pub max_update_frames: usize,
    /// Maximum stderr bytes retained for a bounded, redacted diagnostic.
    pub max_stderr_bytes: usize,
    /// Deadline to spawn + initialize + open a session.
    pub handshake_deadline: Duration,
    /// Deadline for the prompt turn (from `session/prompt` to terminal frame).
    pub prompt_deadline: Duration,
    /// Grace after EOF (stdin close) before escalating to `SIGTERM`.
    pub eof_grace: Duration,
    /// Grace after `SIGTERM` before escalating to `SIGKILL`.
    pub term_grace: Duration,
}

impl AcpLimits {
    /// Validate that every bound is positive and finite. Fails closed on any
    /// zero or absurd value so an unbounded configuration cannot be published.
    fn validate(&self) -> Result<(), AcpConfigError> {
        let byte_bounds = [
            self.max_line_bytes,
            self.max_output_bytes,
            self.max_update_frames,
            self.max_stderr_bytes,
        ];
        if byte_bounds.contains(&0) {
            return Err(AcpConfigError::InvalidLimits);
        }
        let durations = [
            self.handshake_deadline,
            self.prompt_deadline,
            self.eof_grace,
            self.term_grace,
        ];
        if durations.iter().any(Duration::is_zero) {
            return Err(AcpConfigError::InvalidLimits);
        }
        Ok(())
    }
}

/// Immutable, validated configuration for the DSH ACP provider.
///
/// All paths are absolute and validated at construction; the secret is borrowed
/// only for the spawn and is never retained here.
#[derive(Clone)]
pub struct AcpProviderConfig {
    id: ProviderId,
    capability: String,
    generation: i64,
    command: PathBuf,
    args: Vec<String>,
    patch_path: PathBuf,
    dsh_home: PathBuf,
    workspace: PathBuf,
    path_env: OsString,
    permission: PermissionPolicy,
    limits: AcpLimits,
    /// Pinned upstream source commit; must equal `ACP_SOURCE_COMMIT`.
    source_commit: String,
    /// Pinned upstream source version; must equal `ACP_SOURCE_VERSION`.
    source_version: String,
    /// Expected lowercase 64-hex SHA-256 of the on-disk command executable.
    command_sha256: String,
    /// Expected lowercase 64-hex SHA-256 of the on-disk audited patch file.
    patch_sha256: String,
    /// The canonical compile-time audited patch digest, owned by the Runtime
    /// config owner (`runtime-host`) and threaded down verbatim. The on-disk
    /// patch must hash to exactly this value at spawn as well as at
    /// composition; there is no second, independently computed audited digest.
    audited_patch_sha256: String,
    /// The MiniMax API key, materialized into exactly one child env var.
    api_key: String,
}

impl fmt::Debug for AcpProviderConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never render the secret, and keep paths out of Debug so a leaked
        // diagnostic cannot expose the deployment layout.
        formatter
            .debug_struct("AcpProviderConfig")
            .field("id", &self.id)
            .field("capability", &self.capability)
            .field("generation", &self.generation)
            .field("permission", &self.permission)
            .field("limits", &self.limits)
            .field("api_key", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Typed, safe construction failure for the ACP provider config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcpConfigError {
    /// A path was not absolute, or contained a NUL / traversal component.
    InvalidPath,
    /// The command/args/model/route were invalid.
    InvalidRoute,
    /// A resource bound was zero or otherwise not positive-finite.
    InvalidLimits,
    /// The credential was empty or contained a non-environment byte.
    InvalidCredential,
    /// The pinned upstream source commit did not match `ACP_SOURCE_COMMIT`.
    SourceCommitMismatch,
    /// The pinned upstream source version did not match `ACP_SOURCE_VERSION`.
    SourceVersionMismatch,
    /// The argv was not exactly `--profile acp --patch <patch_path>`.
    InvalidArgv,
    /// A configured digest was not a lowercase 64-hex SHA-256 string.
    InvalidDigest,
}

/// Builder inputs for [`AcpProviderConfig::new`]. Every field is validated.
#[derive(Debug)]
pub struct AcpConfigInput<'a> {
    pub id: ProviderId,
    pub capability: &'a str,
    pub generation: i64,
    pub command: &'a str,
    pub args: &'a [String],
    pub patch_path: &'a str,
    pub dsh_home: &'a str,
    pub workspace: &'a str,
    pub path_env: &'a str,
    pub permission: PermissionPolicy,
    pub limits: AcpLimits,
    /// Pinned upstream source commit; must equal [`ACP_SOURCE_COMMIT`].
    pub source_commit: &'a str,
    /// Pinned upstream source version; must equal [`ACP_SOURCE_VERSION`].
    pub source_version: &'a str,
    /// Expected lowercase 64-hex SHA-256 of the on-disk command executable.
    pub command_sha256: &'a str,
    /// Expected lowercase 64-hex SHA-256 of the on-disk audited patch file.
    pub patch_sha256: &'a str,
    /// The canonical compile-time audited patch digest owned by the Runtime
    /// config owner. Must be a lowercase 64-hex SHA-256. The on-disk patch is
    /// compared to this exact value at spawn; there is no duplicate audited
    /// digest computed inside the provider.
    pub audited_patch_sha256: &'a str,
    pub api_key: &'a str,
}

impl AcpProviderConfig {
    /// Validate and build an immutable ACP provider config. All of `command`,
    /// `patch_path`, `dsh_home`, and `workspace` must be absolute paths without
    /// interior NUL bytes; every limit must be positive-finite; the credential
    /// must be a non-empty single-line value.
    pub fn new(input: AcpConfigInput<'_>) -> Result<Self, AcpConfigError> {
        input.limits.validate()?;
        let command = validated_absolute(input.command)?;
        let patch_path = validated_absolute(input.patch_path)?;
        let dsh_home = validated_absolute(input.dsh_home)?;
        let workspace = validated_absolute(input.workspace)?;
        // The upstream source identity is pinned; a mismatch is not a warning,
        // it fails closed. There is no floating branch or `latest` follow.
        if input.source_commit != ACP_SOURCE_COMMIT {
            return Err(AcpConfigError::SourceCommitMismatch);
        }
        if input.source_version != ACP_SOURCE_VERSION {
            return Err(AcpConfigError::SourceVersionMismatch);
        }
        // The argv must be exactly `--profile acp --patch <patch_path>`, where
        // the final element is the same validated absolute patch path. No
        // omission, extra, duplicate, reorder, or mismatch is tolerated.
        if !argv_is_exact(input.args, &patch_path) {
            return Err(AcpConfigError::InvalidArgv);
        }
        for arg in input.args {
            if arg.as_bytes().contains(&0) {
                return Err(AcpConfigError::InvalidRoute);
            }
        }
        // Both digests must be well-formed lowercase 64-hex SHA-256 strings.
        // The actual on-disk comparison happens at Runtime composition; here we
        // only accept a syntactically valid expected digest.
        if !is_lowercase_sha256_hex(input.command_sha256)
            || !is_lowercase_sha256_hex(input.patch_sha256)
            || !is_lowercase_sha256_hex(input.audited_patch_sha256)
        {
            return Err(AcpConfigError::InvalidDigest);
        }
        if input.path_env.is_empty() || input.path_env.as_bytes().contains(&0) {
            return Err(AcpConfigError::InvalidPath);
        }
        if input.capability.trim().is_empty() || input.generation < 0 {
            return Err(AcpConfigError::InvalidRoute);
        }
        if !valid_credential(input.api_key) {
            return Err(AcpConfigError::InvalidCredential);
        }
        Ok(Self {
            id: input.id,
            capability: input.capability.to_owned(),
            generation: input.generation,
            command,
            args: input.args.to_vec(),
            patch_path,
            dsh_home,
            workspace,
            path_env: OsString::from(input.path_env),
            permission: input.permission,
            limits: input.limits,
            source_commit: input.source_commit.to_owned(),
            source_version: input.source_version.to_owned(),
            command_sha256: input.command_sha256.to_owned(),
            patch_sha256: input.patch_sha256.to_owned(),
            audited_patch_sha256: input.audited_patch_sha256.to_owned(),
            api_key: input.api_key.to_owned(),
        })
    }

    /// The pinned upstream source commit this config was built against.
    pub fn source_commit(&self) -> &str {
        &self.source_commit
    }

    /// The pinned upstream source version this config was built against.
    pub fn source_version(&self) -> &str {
        &self.source_version
    }

    /// The expected lowercase 64-hex SHA-256 of the on-disk command executable.
    pub fn command_sha256(&self) -> &str {
        &self.command_sha256
    }

    /// The expected lowercase 64-hex SHA-256 of the on-disk audited patch file.
    pub fn patch_sha256(&self) -> &str {
        &self.patch_sha256
    }

    /// The canonical compile-time audited patch digest threaded from the
    /// Runtime config owner. The on-disk patch must hash to exactly this value.
    pub fn audited_patch_sha256(&self) -> &str {
        &self.audited_patch_sha256
    }

    /// The validated absolute command executable path.
    pub fn command_path(&self) -> &std::path::Path {
        &self.command
    }

    /// The validated absolute audited patch path.
    pub fn patch_file(&self) -> &std::path::Path {
        &self.patch_path
    }

    /// Re-validate the on-disk spawn assets immediately before `Command::new`,
    /// closing the time-of-check/time-of-use gap between Runtime composition
    /// (which validated the same assets before resolving the secret) and the
    /// actual `spawn`.
    ///
    /// This is a *production* check: it always runs on the live spawn path, for
    /// tests and deployment alike. It re-opens the command and the audited
    /// patch through [`std::fs::symlink_metadata`] (never following a symlink),
    /// requires each to be a direct regular file with a bounded size and the
    /// correct permission bits (owner-execute for the command; neither group-
    /// nor other-writable for either), streams a fresh SHA-256, and compares it
    /// in constant time to the configured digests. The patch is *additionally*
    /// compared to the canonical audited digest threaded from the Runtime owner
    /// ([`Self::audited_patch_sha256`]), so a swapped-but-syntactically-valid
    /// patch is rejected here just as it is at composition.
    ///
    /// It then walks every ancestor directory of both assets and rejects a
    /// symlinked or group/other-writable ancestor below a *trusted barrier*
    /// (see [`verify_ancestor_chain`] for the exact rule), so an attacker who
    /// only controls a writable intermediate directory cannot swap a validated
    /// leaf between check and use.
    ///
    /// Every failure maps to a single safe, category-only [`ExecuteError`]:
    /// no path, digest, size, mode, or uid ever appears in the error. A
    /// mutated, replaced, symlinked, or wrongly-permissioned asset — or a
    /// writable ancestor below the barrier — fails closed here, before any
    /// child is spawned.
    pub fn verify_spawn_assets(&self) -> Result<(), ExecuteError> {
        // Command: direct regular file, owner-executable, not group/other
        // writable, bounded, digest == configured command digest.
        let command_digest = verify_regular_file(
            &self.command,
            MAX_SPAWN_COMMAND_BYTES,
            SpawnFileRule::Executable,
        )?;
        if !constant_time_hex_eq(&command_digest, &self.command_sha256) {
            return Err(ExecuteError::ExecutionFailed);
        }

        // Patch: direct regular file, not group/other writable, bounded, digest
        // == BOTH the configured value AND the canonical audited digest.
        let patch_digest =
            verify_regular_file(&self.patch_path, MAX_SPAWN_PATCH_BYTES, SpawnFileRule::Data)?;
        if !constant_time_hex_eq(&patch_digest, &self.patch_sha256) {
            return Err(ExecuteError::ExecutionFailed);
        }
        if !constant_time_hex_eq(&patch_digest, &self.audited_patch_sha256) {
            return Err(ExecuteError::ExecutionFailed);
        }

        // Every ancestor directory of both assets must be safe up to a trusted
        // barrier, so a writable intermediate cannot swap the validated leaf.
        verify_ancestor_chain(&self.command)?;
        verify_ancestor_chain(&self.patch_path)?;
        Ok(())
    }

    /// A safe, secret-free one-line description for doctor/audit evidence. It
    /// intentionally omits paths, args, and the credential; only the pinned
    /// route, permission policy, and patch presence are reported.
    pub fn describe(&self) -> String {
        format!(
            "deepseek-harness-acp route={ACP_ROUTE_PROVIDER}/{ACP_ROUTE_MODEL} permission={} patch={} isolated-dsh-home",
            self.permission.as_str(),
            if self.patch_path.as_os_str().is_empty() {
                "absent"
            } else {
                "present"
            }
        )
    }
}

fn validated_absolute(raw: &str) -> Result<PathBuf, AcpConfigError> {
    if raw.is_empty() || raw.as_bytes().contains(&0) {
        return Err(AcpConfigError::InvalidPath);
    }
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err(AcpConfigError::InvalidPath);
    }
    // Reject relative traversal components; an absolute path must be canonical
    // enough that no `..` component can climb out of a validated root.
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(AcpConfigError::InvalidPath);
    }
    Ok(path)
}

/// Verify the argv is exactly `--profile acp --patch <patch_path>`, where the
/// final element equals the validated absolute patch path. Any omission, extra,
/// duplicate, reorder, or path mismatch fails closed.
fn argv_is_exact(args: &[String], patch_path: &std::path::Path) -> bool {
    let patch = match patch_path.to_str() {
        Some(patch) => patch,
        None => return false,
    };
    args.len() == 4
        && args[0] == ACP_ARG_PROFILE_FLAG
        && args[1] == ACP_ARG_PROFILE_VALUE
        && args[2] == ACP_ARG_PATCH_FLAG
        && args[3] == patch
}

/// True iff `value` is exactly 64 lowercase hexadecimal digits (a SHA-256).
fn is_lowercase_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_credential(secret: &str) -> bool {
    !secret.is_empty()
        && secret.len() <= 4096
        && !secret
            .bytes()
            .any(|byte| byte == 0 || byte == b'\n' || byte == b'\r')
}

/// Whether a spawn-time asset must additionally be owner-executable.
#[derive(Clone, Copy)]
enum SpawnFileRule {
    Executable,
    Data,
}

/// Re-open a spawn asset via `symlink_metadata` (never following a symlink),
/// enforce that it is a direct regular file, neither group- nor other-writable,
/// optionally owner-executable, and within `max_bytes`, then stream a fresh
/// SHA-256 and return it as lowercase 64-hex. All failures collapse to a single
/// safe, category-only [`ExecuteError`] that carries no path, mode, or size.
fn verify_regular_file(
    path: &std::path::Path,
    max_bytes: u64,
    rule: SpawnFileRule,
) -> Result<String, ExecuteError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| ExecuteError::ExecutionFailed)?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || !file_type.is_file() {
        return Err(ExecuteError::ExecutionFailed);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        // Reject any group- or other-writable file.
        if mode & 0o022 != 0 {
            return Err(ExecuteError::ExecutionFailed);
        }
        if let SpawnFileRule::Executable = rule {
            if mode & 0o100 == 0 {
                return Err(ExecuteError::ExecutionFailed);
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = rule;
    }
    if metadata.len() > max_bytes {
        return Err(ExecuteError::ExecutionFailed);
    }
    stream_sha256_hex(path, max_bytes)
}

/// Stream a file's SHA-256, refusing to read past `max_bytes` (a file that
/// grew between the stat and the read fails closed rather than being hashed
/// unboundedly). Returns lowercase 64-hex.
fn stream_sha256_hex(path: &std::path::Path, max_bytes: u64) -> Result<String, ExecuteError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|_| ExecuteError::ExecutionFailed)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; SPAWN_HASH_CHUNK_BYTES];
    let mut total: u64 = 0;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| ExecuteError::ExecutionFailed)?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        if total > max_bytes {
            return Err(ExecuteError::ExecutionFailed);
        }
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

/// Length-checked byte-diff equality that does not early-return on the first
/// differing byte. Both inputs are lowercase 64-hex SHA-256 strings.
fn constant_time_hex_eq(actual: &str, expected: &str) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (a, b) in actual.bytes().zip(expected.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// Validate every ancestor directory of `leaf` against symlink and writability
/// attacks, up to a *trusted barrier*.
///
/// # The exact rule
///
/// Starting from the leaf's parent and walking upward, each ancestor must be a
/// direct directory (never a symlink). The walk *stops accepting* at the first
/// **trusted barrier**, which is either:
///
/// 1. the filesystem root reached through a chain of ancestors none of which
///    were group- or other-writable; or
/// 2. an **owner-only** directory — mode has no group/other bits at all
///    (`mode & 0o077 == 0`, i.e. a `0700`-equivalent) — that is owned by the
///    current **effective uid**.
///
/// Below the barrier, any ancestor that is group- or other-writable is
/// rejected. This is what lets a `mkdtemp` `0700` root created under the sticky
/// world-writable `/tmp` be trusted: the `0700` owner-owned root is itself the
/// barrier, so the world-writable `/tmp` above it is never inspected. A
/// group/other-writable directory *below* such a barrier (or below root)
/// always fails closed.
///
/// A symlinked ancestor is always rejected. Every failure is the same safe,
/// category-only [`ExecuteError`]; no path, mode, or uid is surfaced.
fn verify_ancestor_chain(leaf: &std::path::Path) -> Result<(), ExecuteError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        use std::os::unix::fs::PermissionsExt;

        let effective_uid = nix::unistd::geteuid().as_raw();
        let mut current = leaf.parent();
        while let Some(dir) = current {
            // `symlink_metadata` so a symlinked ancestor is caught (not
            // silently followed to a different, attacker-chosen target).
            let metadata =
                std::fs::symlink_metadata(dir).map_err(|_| ExecuteError::ExecutionFailed)?;
            let file_type = metadata.file_type();
            if file_type.is_symlink() || !file_type.is_dir() {
                return Err(ExecuteError::ExecutionFailed);
            }
            let mode = metadata.permissions().mode();
            let owner_only = mode & 0o077 == 0;
            let owned_by_effective = metadata.uid() == effective_uid;
            // Barrier 2: an owner-only directory owned by the effective uid is
            // trusted; stop here and accept everything above it.
            if owner_only && owned_by_effective {
                return Ok(());
            }
            // Below the barrier, a group/other-writable ancestor is a swap
            // vector and fails closed.
            if mode & 0o022 != 0 {
                return Err(ExecuteError::ExecutionFailed);
            }
            let parent = dir.parent();
            // Barrier 1: reached the filesystem root through non-writable
            // ancestors (parent is None, or parent equals self at root).
            match parent {
                None => return Ok(()),
                Some(next) if next == dir => return Ok(()),
                Some(next) => current = Some(next),
            }
        }
        // A leaf with no parent (e.g. bare root) has no ancestor to attack.
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = leaf;
        Ok(())
    }
}

/// The production DeepSeek Harness ACP provider.
#[derive(Debug)]
pub struct DeepSeekHarnessAcpProvider {
    config: AcpProviderConfig,
}

impl DeepSeekHarnessAcpProvider {
    pub fn new(config: AcpProviderConfig) -> Self {
        Self { config }
    }
}

impl ExecuteProvider for DeepSeekHarnessAcpProvider {
    fn id(&self) -> &ProviderId {
        &self.config.id
    }

    fn capability(&self) -> &str {
        &self.config.capability
    }

    fn generation(&self) -> i64 {
        self.config.generation
    }

    fn execute<'a>(
        &'a self,
        request: &'a ExecuteRequest,
        cancellation: &'a CancellationToken,
    ) -> ExecuteFuture<'a> {
        Box::pin(async move {
            let (outcome, tool_effects) =
                run_acp_session(&self.config, request, cancellation).await?;
            Ok(ExecuteResult {
                provider_id: self.config.id.clone(),
                provider_generation: self.config.generation,
                outcome,
                tool_effects,
            })
        })
    }
}

/// Drive one full ACP session for one execution attempt on a fresh process.
///
/// Returns the terminal outcome plus the bounded, provider-neutral per-tool
/// side-effect dispositions observed during the turn (`DXB-DEL-068` H10). Even
/// on an error path the effects observed so far are meaningful to the caller's
/// side-effect ledger, but the ACP terminal contract already collapses errors
/// to a typed `ExecuteError`; effects therefore travel only on the success and
/// cancellation paths where a bounded result is produced.
async fn run_acp_session(
    config: &AcpProviderConfig,
    request: &ExecuteRequest,
    cancellation: &CancellationToken,
) -> Result<(ExecuteOutcome, Vec<ToolEffectRecord>), ExecuteError> {
    if cancellation.is_cancelled() {
        return Ok((ExecuteOutcome::Cancelled, Vec::new()));
    }
    let mut session = AcpSession::spawn(config)?;
    // Ensure the child is always reaped, whatever path we exit on.
    let result = session.drive(config, request, cancellation).await;
    session.dispose(config).await;
    result.map(|outcome| (outcome, session.take_tool_effects()))
}

/// Maximum number of distinct tool correlations tracked per session. Bounds the
/// per-tool bookkeeping so a malicious/looping agent cannot grow it unbounded.
const MAX_TOOL_CORRELATIONS: usize = 256;
/// Maximum bytes retained for a bounded, opaque correlation id.
const MAX_CORRELATION_BYTES: usize = 128;

/// A live ACP child and its bounded protocol state.
struct AcpSession {
    child: Child,
    reader: BufReader<tokio::process::ChildStdout>,
    stderr: Option<tokio::process::ChildStderr>,
    stdin: Option<tokio::process::ChildStdin>,
    next_id: i64,
    session_id: Option<String>,
    line_buf: String,
    /// Whether `initialize` advertised `sessionCapabilities.close`. Only then
    /// does teardown attempt a best-effort graceful `session/close`; the
    /// EOF → TERM → KILL ladder remains the authoritative fallback.
    close_supported: bool,
    /// The child's process-group id (pgid == child pid), captured at spawn so
    /// teardown can always reap the whole group — including grandchildren that
    /// outlive the direct child after a graceful/EOF exit.
    pgid: Option<i32>,
    /// Bounded set of observed tool correlations (opaque adapter-local ids), so
    /// a tool-effect update can be attributed to a prior tool-call start. An
    /// effect for an unknown correlation is unattributed and fails protocol.
    tool_correlations: std::collections::BTreeSet<String>,
    /// Bounded, provider-neutral per-tool side-effect dispositions.
    tool_effects: Vec<ToolEffectRecord>,
}

impl AcpSession {
    /// Take the accumulated bounded tool effects, leaving the session's list
    /// empty. Called once after the drive completes.
    fn take_tool_effects(&mut self) -> Vec<ToolEffectRecord> {
        std::mem::take(&mut self.tool_effects)
    }

    /// Record a bounded, provider-neutral tool-effect disposition, enforcing the
    /// per-session correlation bound. Returns a protocol violation if the
    /// bounded correlation budget is exceeded (a runaway agent).
    fn record_tool_effect(
        &mut self,
        correlation: &str,
        disposition: ToolDisposition,
    ) -> Result<(), ExecuteError> {
        if self.tool_effects.len() >= MAX_TOOL_CORRELATIONS {
            return Err(ExecuteError::OutputExceeded);
        }
        self.tool_effects.push(ToolEffectRecord {
            correlation: bounded_label(correlation, MAX_CORRELATION_BYTES),
            disposition,
        });
        Ok(())
    }

    fn spawn(config: &AcpProviderConfig) -> Result<Self, ExecuteError> {
        // Re-validate the on-disk command and audited patch immediately before
        // constructing the child, closing the composition→spawn TOCTOU gap.
        // A mutated, replaced, symlinked, wrongly-permissioned asset, or a
        // writable ancestor below the trusted barrier, fails closed here with a
        // safe category-only error — before any process is created.
        config.verify_spawn_assets()?;
        let mut command = Command::new(&config.command);
        command
            .args(&config.args)
            .current_dir(&config.workspace)
            .env_clear()
            .env(ENV_PATH, &config.path_env)
            .env(ENV_DSH_HOME, &config.dsh_home)
            .env(ENV_MINIMAX_API_KEY, &config.api_key)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        // Put the child in its own process group so teardown can signal the
        // whole tree (child + grandchildren). `process_group` is a safe stable
        // API; the negative-pgid signalling happens in `dispose`.
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command
            .spawn()
            .map_err(|_| ExecuteError::TransportUnavailable)?;
        // Capture the process-group id now, while the child is guaranteed live.
        // The child was spawned into its own group (pgid == pid), so this pgid
        // names the whole tree (child + any grandchildren) for teardown, even
        // after the direct child has already exited.
        let pgid = child.id().and_then(|pid| i32::try_from(pid).ok());
        let stdout = child
            .stdout
            .take()
            .ok_or(ExecuteError::TransportUnavailable)?;
        let stderr = child.stderr.take();
        let stdin = child.stdin.take();
        Ok(Self {
            child,
            reader: BufReader::new(stdout),
            stderr,
            stdin,
            next_id: 1,
            session_id: None,
            line_buf: String::new(),
            close_supported: false,
            pgid,
            tool_correlations: std::collections::BTreeSet::new(),
            tool_effects: Vec::new(),
        })
    }

    fn allocate_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// Full ordered drive: initialize → session/new → session/prompt, consuming
    /// only matching `session/update`, answering permission requests, and
    /// mapping the terminal `stopReason`.
    async fn drive(
        &mut self,
        config: &AcpProviderConfig,
        request: &ExecuteRequest,
        cancellation: &CancellationToken,
    ) -> Result<ExecuteOutcome, ExecuteError> {
        let handshake = config.limits.handshake_deadline;
        // 1. initialize
        let init_id = self.allocate_id();
        self.write_request(
            config,
            init_id,
            "initialize",
            json!({
                "protocolVersion": ACP_PROTOCOL_VERSION,
                "clientCapabilities": {}
            }),
        )
        .await?;
        let Some(init) = self
            .await_response(config, init_id, handshake, cancellation)
            .await?
        else {
            return Ok(ExecuteOutcome::Cancelled);
        };
        let capabilities = verify_initialize(&init)?;
        self.close_supported = capabilities.close;

        // 2. session/new with absolute cwd and empty mcpServers
        if cancellation.is_cancelled() {
            return Ok(ExecuteOutcome::Cancelled);
        }
        let workspace = config
            .workspace
            .to_str()
            .ok_or(ExecuteError::ExecutionFailed)?;
        let new_id = self.allocate_id();
        self.write_request(
            config,
            new_id,
            "session/new",
            json!({
                "cwd": workspace,
                "mcpServers": []
            }),
        )
        .await?;
        let Some(new_session) = self
            .await_response(config, new_id, handshake, cancellation)
            .await?
        else {
            return Ok(ExecuteOutcome::Cancelled);
        };
        let session_id = extract_session_id(&new_session)?;
        self.session_id = Some(session_id.clone());

        // Verify the advertised route BEFORE publishing any provider activity.
        // The ACP `NewSessionResponse.configOptions` array advertises the model
        // selector; its `currentValue` must be exactly `["minimax","MiniMax-M3"]`.
        // Any drift, or an absent model option, fails closed here — no prompt is
        // ever sent to a mis-routed session.
        let config_options = new_session
            .get("result")
            .and_then(|result| result.get("configOptions"))
            .ok_or(ExecuteError::ProtocolViolation)?;
        verify_model_route(config_options)?;

        // 3. session/prompt (ordered text), consume matching session/update.
        if cancellation.is_cancelled() {
            self.cancel_session(config, &session_id).await;
            return Ok(ExecuteOutcome::Cancelled);
        }
        let prompt_id = self.allocate_id();
        let prompt_text = format!("{}\n\nContext: {}", request.intent, request.context);
        self.write_request(
            config,
            prompt_id,
            "session/prompt",
            json!({
                "sessionId": session_id,
                "prompt": [{ "type": "text", "text": prompt_text }]
            }),
        )
        .await?;

        self.consume_prompt(config, request, &session_id, prompt_id, cancellation)
            .await
    }

    /// Consume `session/update` frames and permission requests until the prompt
    /// response arrives, a deadline elapses, or cancellation is observed.
    async fn consume_prompt(
        &mut self,
        config: &AcpProviderConfig,
        request: &ExecuteRequest,
        session_id: &str,
        prompt_id: i64,
        cancellation: &CancellationToken,
    ) -> Result<ExecuteOutcome, ExecuteError> {
        let deadline = Instant::now() + config.limits.prompt_deadline;
        let mut output = String::new();
        let mut frames = 0_usize;
        loop {
            if cancellation.is_cancelled() {
                self.cancel_session(config, session_id).await;
                return Ok(ExecuteOutcome::Cancelled);
            }
            let remaining = deadline.checked_duration_since(Instant::now());
            let Some(remaining) = remaining else {
                self.cancel_session(config, session_id).await;
                return Err(ExecuteError::DeadlineExceeded);
            };
            let message = match self.read_message(config, remaining).await {
                Ok(Some(message)) => message,
                Ok(None) => return Err(ExecuteError::IncompleteStream),
                Err(error) => return Err(error),
            };
            // A JSON-RPC response for the prompt id is the terminal signal.
            if message_id(&message) == Some(prompt_id) {
                if let Some(error) = message.get("error") {
                    return Err(classify_rpc_error(error));
                }
                let stop = message
                    .get("result")
                    .and_then(|result| result.get("stopReason"))
                    .and_then(Value::as_str)
                    .ok_or(ExecuteError::ProtocolViolation)?;
                return finalize_stop_reason(stop, output);
            }
            // Server-initiated request: session/request_permission.
            if let Some(method) = message.get("method").and_then(Value::as_str) {
                match method {
                    "session/request_permission" => {
                        self.answer_permission(config, request, session_id, &message)
                            .await?;
                        continue;
                    }
                    "session/update" => {
                        frames = frames.saturating_add(1);
                        if frames > config.limits.max_update_frames {
                            self.cancel_session(config, session_id).await;
                            return Err(ExecuteError::OutputExceeded);
                        }
                        // Map tool lifecycle to bounded, provider-neutral
                        // effects (`DXB-DEL-068` H10 Task 6/8). A malformed or
                        // unattributed tool update fails protocol and never
                        // grants; assistant text is accumulated as output.
                        self.consume_update(&message, session_id, config, &mut output)?;
                        continue;
                    }
                    // Any other server method is outside the P0 automation
                    // surface; consume it without acting (no MCP injection, no
                    // fs/terminal). It is neither authority nor output.
                    _ => continue,
                }
            }
            // A response id we never issued, or an unknown session id, is a
            // protocol violation — we never guess an association.
            if message.get("id").is_some() {
                return Err(ExecuteError::ProtocolViolation);
            }
        }
    }

    /// Answer a `session/request_permission`.
    ///
    /// The static [`PermissionPolicy`] is only an *upper bound*. An
    /// `allow_once` option is selected **only** when both hold:
    ///
    /// 1. the config's policy is `AllowOnce` (the deployment permits it at
    ///    all), and
    /// 2. the request carries a valid request-scoped [`AllowOnceGrant`]
    ///    (`request.allow_once_grant`) — a DXBOT runtime-security authority the
    ///    scheduler minted from an Approved Approval bound to
    ///    `provider-tool-allow-once` for exactly this execution.
    ///
    /// Absent that authority — or under the default `Reject` policy — every
    /// permission request is cancelled. A malformed/unattributed request (no
    /// id, wrong session, missing options) fails protocol and never grants. A
    /// DSH/ambient approval is never trusted as authority.
    async fn answer_permission(
        &mut self,
        config: &AcpProviderConfig,
        request: &ExecuteRequest,
        session_id: &str,
        message: &Value,
    ) -> Result<(), ExecuteError> {
        let request_id = message
            .get("id")
            .cloned()
            .ok_or(ExecuteError::ProtocolViolation)?;
        // The permission request must address our exact session id.
        let params = message
            .get("params")
            .ok_or(ExecuteError::ProtocolViolation)?;
        if params.get("sessionId").and_then(Value::as_str) != Some(session_id) {
            return Err(ExecuteError::ProtocolViolation);
        }

        // Record the permission request as a bounded, neutral effect keyed on a
        // correlation derived from the tool call id (if any). An unattributed
        // permission request (no toolCallId) is still recorded neutrally but
        // never grants.
        let correlation = params
            .get("toolCall")
            .and_then(|call| call.get("toolCallId"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("permission:{request_id}"));

        // The authority gate: allow_once requires BOTH the AllowOnce policy AND
        // a valid request-scoped grant. Everything else is a reject.
        let authorized_allow_once = matches!(config.permission, PermissionPolicy::AllowOnce)
            && request
                .allow_once_grant
                .as_ref()
                .is_some_and(AllowOnceGrant::is_valid);

        let outcome = if authorized_allow_once {
            match select_allow_once_option(params) {
                Some(option) => {
                    self.record_tool_effect(&correlation, ToolDisposition::Dispatched)?;
                    option
                }
                // The policy+authority permitted an allow-once but the agent
                // advertised no such option: reject, never guess.
                None => {
                    self.record_tool_effect(&correlation, ToolDisposition::Rejected)?;
                    json!({ "outcome": "cancelled" })
                }
            }
        } else {
            // Default reject (unattended default, or no authority grant).
            self.record_tool_effect(&correlation, ToolDisposition::Rejected)?;
            json!({ "outcome": "cancelled" })
        };
        self.write_result(config, request_id, json!({ "outcome": outcome }))
            .await
    }

    /// Consume a `session/update` frame, mapping assistant text to output and
    /// tool lifecycle to bounded, provider-neutral effects (`DXB-DEL-068` H10).
    /// A malformed update or one for a foreign session id fails protocol; a
    /// tool-effect update for an unknown correlation is unattributed and also
    /// fails protocol (never silently granted or accumulated).
    fn consume_update(
        &mut self,
        message: &Value,
        session_id: &str,
        config: &AcpProviderConfig,
        output: &mut String,
    ) -> Result<(), ExecuteError> {
        let params = message
            .get("params")
            .ok_or(ExecuteError::ProtocolViolation)?;
        if params.get("sessionId").and_then(Value::as_str) != Some(session_id) {
            return Err(ExecuteError::ProtocolViolation);
        }
        let update = params
            .get("update")
            .ok_or(ExecuteError::ProtocolViolation)?;
        let kind = update
            .get("sessionUpdate")
            .and_then(Value::as_str)
            .ok_or(ExecuteError::ProtocolViolation)?;
        match kind {
            "agent_message_chunk" => {
                if let Some(text) = update
                    .get("content")
                    .filter(|content| content.get("type").and_then(Value::as_str) == Some("text"))
                    .and_then(|content| content.get("text"))
                    .and_then(Value::as_str)
                {
                    if output.len().saturating_add(text.len()) > config.limits.max_output_bytes {
                        return Err(ExecuteError::OutputExceeded);
                    }
                    output.push_str(text);
                }
                Ok(())
            }
            "tool_call" => {
                // A new tool call: register its bounded correlation so later
                // effect updates can be attributed. A tool_call with no id is
                // malformed and fails protocol.
                let correlation = update
                    .get("toolCallId")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty())
                    .ok_or(ExecuteError::ProtocolViolation)?;
                if self.tool_correlations.len() >= MAX_TOOL_CORRELATIONS {
                    return Err(ExecuteError::OutputExceeded);
                }
                self.tool_correlations
                    .insert(bounded_label(correlation, MAX_CORRELATION_BYTES));
                self.record_tool_effect(correlation, ToolDisposition::Prepared)
            }
            "tool_call_update" => {
                // An effect update must attribute to a known tool call. An
                // unknown/absent correlation is unattributed and fails protocol
                // — it never grants and is never recorded as a real effect.
                let correlation = update
                    .get("toolCallId")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty())
                    .ok_or(ExecuteError::ProtocolViolation)?;
                if !self
                    .tool_correlations
                    .contains(&bounded_label(correlation, MAX_CORRELATION_BYTES))
                {
                    return Err(ExecuteError::ProtocolViolation);
                }
                let disposition = match update.get("status").and_then(Value::as_str) {
                    Some("completed") => ToolDisposition::Confirmed,
                    Some("failed") => ToolDisposition::Rejected,
                    Some("in_progress") | None => ToolDisposition::Dispatched,
                    // An unknown status maps to Unknown so the caller reconciles
                    // rather than assuming completion.
                    Some(_) => ToolDisposition::Unknown,
                };
                self.record_tool_effect(correlation, disposition)
            }
            // Thoughts / plans / other neutral updates are neither output nor
            // authority; observe without acting.
            _ => Ok(()),
        }
    }

    /// Best-effort `session/cancel` notification; teardown remains authoritative.
    async fn cancel_session(&mut self, config: &AcpProviderConfig, session_id: &str) {
        let _ = self
            .write_notification(config, "session/cancel", json!({ "sessionId": session_id }))
            .await;
    }

    /// Write a JSON-RPC 2.0 request line. Bounds the outgoing line length.
    async fn write_request(
        &mut self,
        config: &AcpProviderConfig,
        id: i64,
        method: &str,
        params: Value,
    ) -> Result<(), ExecuteError> {
        let frame = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.write_frame(config, &frame).await
    }

    async fn write_result(
        &mut self,
        config: &AcpProviderConfig,
        id: Value,
        result: Value,
    ) -> Result<(), ExecuteError> {
        let frame = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });
        self.write_frame(config, &frame).await
    }

    async fn write_notification(
        &mut self,
        config: &AcpProviderConfig,
        method: &str,
        params: Value,
    ) -> Result<(), ExecuteError> {
        let frame = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        self.write_frame(config, &frame).await
    }

    async fn write_frame(
        &mut self,
        config: &AcpProviderConfig,
        frame: &Value,
    ) -> Result<(), ExecuteError> {
        let mut line = serde_json::to_string(frame).map_err(|_| ExecuteError::ExecutionFailed)?;
        if line.len().saturating_add(1) > config.limits.max_line_bytes {
            return Err(ExecuteError::OutputExceeded);
        }
        line.push('\n');
        let stdin = self
            .stdin
            .as_mut()
            .ok_or(ExecuteError::TransportUnavailable)?;
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|_| ExecuteError::TransportUnavailable)?;
        stdin
            .flush()
            .await
            .map_err(|_| ExecuteError::TransportUnavailable)?;
        Ok(())
    }

    /// Await the JSON-RPC response for a specific request id within a deadline,
    /// consuming and ignoring unrelated notifications (but rejecting foreign
    /// response ids as protocol violations). `Ok(None)` means caller
    /// cancellation was observed and must become `ExecuteOutcome::Cancelled`.
    ///
    /// This drives only the **handshake** requests (`initialize`,
    /// `session/new`). A clean EOF here means the spawned child exited *before*
    /// completing the handshake — the ACP server never came up (a startup crash
    /// such as an unresolved runtime/module tree, a missing package root, or an
    /// immediate non-zero exit). That is a transport/startup failure, not a
    /// mid-turn truncation, so it maps to [`ExecuteError::TransportUnavailable`]
    /// (surfaced as `transport-unavailable`) rather than the generic
    /// [`ExecuteError::IncompleteStream`] (`execution-failed`). Collapsing a
    /// never-started child into `execution-failed` hid exactly this class of
    /// deployment defect: the child produced no protocol frame at all, yet the
    /// failure looked like an opaque execution error with no output.
    async fn await_response(
        &mut self,
        config: &AcpProviderConfig,
        id: i64,
        deadline: Duration,
        cancellation: &CancellationToken,
    ) -> Result<Option<Value>, ExecuteError> {
        let deadline_at = Instant::now() + deadline;
        loop {
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            let remaining = deadline_at
                .checked_duration_since(Instant::now())
                .ok_or(ExecuteError::DeadlineExceeded)?;
            let read = tokio::select! {
                () = cancellation.cancelled() => return Ok(None),
                read = self.read_message(config, remaining) => read,
            };
            let message = match read {
                Ok(Some(message)) => message,
                // EOF during the handshake: the child never produced its
                // `initialize`/`session/new` response. Treat a never-started
                // ACP server as a transport/startup failure.
                Ok(None) => return Err(ExecuteError::TransportUnavailable),
                Err(error) => return Err(error),
            };
            match message_id(&message) {
                Some(observed) if observed == id => {
                    if let Some(error) = message.get("error") {
                        return Err(classify_rpc_error(error));
                    }
                    return Ok(Some(message));
                }
                // A response bearing an id we never issued: never guess.
                Some(_) => return Err(ExecuteError::ProtocolViolation),
                // A notification during handshake (no id): ignore and continue.
                None => continue,
            }
        }
    }

    /// Read exactly one newline-delimited JSON value with bounded line length
    /// and a read deadline. `Ok(None)` means clean EOF.
    async fn read_message(
        &mut self,
        config: &AcpProviderConfig,
        deadline: Duration,
    ) -> Result<Option<Value>, ExecuteError> {
        self.line_buf.clear();
        let read = timeout(
            deadline,
            read_bounded_line(
                &mut self.reader,
                &mut self.line_buf,
                config.limits.max_line_bytes,
            ),
        )
        .await;
        let bytes = match read {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(error)) => return Err(error),
            Err(_) => return Err(ExecuteError::DeadlineExceeded),
        };
        if bytes == 0 {
            return Ok(None);
        }
        let trimmed = self.line_buf.trim_end_matches(['\n', '\r']);
        if trimmed.is_empty() {
            // A blank line is valid framing filler; treat as skippable.
            return Ok(Some(json!({ "jsonrpc": "2.0" })));
        }
        // stdout must carry only JSON-RPC frames. Any non-JSON line is
        // contamination and fails closed.
        let value: Value =
            serde_json::from_str(trimmed).map_err(|_| ExecuteError::ProtocolViolation)?;
        if value.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Err(ExecuteError::ProtocolViolation);
        }
        Ok(Some(value))
    }

    /// Bounded teardown: EOF (drop stdin) → SIGTERM → SIGKILL to the whole
    /// process group, then reap. Also drains a bounded amount of stderr into a
    /// redacted diagnostic sink that is intentionally discarded here.
    async fn dispose(&mut self, config: &AcpProviderConfig) {
        // If the agent advertised `sessionCapabilities.close` and we opened a
        // session, request a graceful `session/close` first. This is strictly
        // best-effort and bounded by the EOF grace; the EOF → TERM → KILL
        // ladder below stays authoritative regardless of the outcome.
        if self.close_supported {
            if let Some(session_id) = self.session_id.clone() {
                let close_id = self.allocate_id();
                let _ = timeout(
                    config.limits.eof_grace,
                    self.write_request(
                        config,
                        close_id,
                        "session/close",
                        json!({ "sessionId": session_id }),
                    ),
                )
                .await;
            }
        }
        // Drop stdin to signal EOF so a cooperative child can flush and exit.
        self.stdin = None;
        // Drain bounded stderr so the pipe cannot wedge the child; the content
        // is intentionally not surfaced (no raw stderr leaves the boundary).
        if let Some(mut stderr) = self.stderr.take() {
            let mut sink = vec![0_u8; config.limits.max_stderr_bytes.min(64 * 1024)];
            let _ = timeout(config.limits.eof_grace, stderr.read(&mut sink)).await;
        }
        // EOF grace: a cooperative child flushes and exits. Even if it does,
        // grandchildren it spawned may still be alive in the group, so the
        // final group SIGKILL sweep below always runs.
        if !wait_exit(&mut self.child, config.limits.eof_grace).await {
            // SIGTERM to the process group, then a bounded grace.
            signal_group(self.pgid, TeardownSignal::Term);
            if !wait_exit(&mut self.child, config.limits.term_grace).await {
                // SIGKILL the group and reap the direct child.
                signal_group(self.pgid, TeardownSignal::Kill);
                let _ = self.child.start_kill();
                let _ = self.child.wait().await;
            }
        }
        // Authoritative sweep: unconditionally SIGKILL the whole process group.
        // This reaps any grandchild that outlived a graceful/EOF child exit, so
        // no child or grandchild can leak past teardown. Signalling an
        // already-empty group is a harmless no-op (ESRCH).
        signal_group(self.pgid, TeardownSignal::Kill);
    }
}

/// Read one line into `buf`, refusing to exceed `max_line_bytes`.
async fn read_bounded_line(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    buf: &mut String,
    max_line_bytes: usize,
) -> Result<usize, ExecuteError> {
    let mut raw = Vec::new();
    // Read byte-oriented so we can enforce the bound before UTF-8 decoding.
    let limit = u64::try_from(max_line_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut limited = reader.take(limit);
    let read = limited
        .read_until(b'\n', &mut raw)
        .await
        .map_err(|_| ExecuteError::TransportUnavailable)?;
    if read == 0 {
        return Ok(0);
    }
    if raw.len() > max_line_bytes {
        return Err(ExecuteError::OutputExceeded);
    }
    let text = String::from_utf8(raw).map_err(|_| ExecuteError::ProtocolViolation)?;
    buf.push_str(&text);
    Ok(read)
}

/// Select a bounded `selected` outcome for the first advertised `allow_once`
/// option. Returns `None` if no such option is advertised (never guesses).
fn select_allow_once_option(params: &Value) -> Option<Value> {
    params
        .get("options")
        .and_then(Value::as_array)
        .and_then(|options| {
            options.iter().find_map(|option| {
                let kind = option.get("kind").and_then(Value::as_str)?;
                if kind == "allow_once" {
                    let id = option.get("optionId").and_then(Value::as_str)?;
                    Some(json!({ "outcome": "selected", "optionId": id }))
                } else {
                    None
                }
            })
        })
}

/// Truncate a label/id to a bounded number of bytes on a UTF-8 boundary, so a
/// runaway external id cannot grow bookkeeping unbounded. Neutral: the result
/// is used only as an opaque correlation/label, never surfaced as a path or
/// argument.
fn bounded_label(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

/// Advertised agent capabilities the adapter relies on, extracted from a
/// verified `initialize` response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AgentCapabilities {
    /// Whether `sessionCapabilities.close` was advertised.
    close: bool,
}

/// Verify the `initialize` response: exact protocol version, empty auth
/// methods, and the required capabilities the adapter relies on. The agent
/// must advertise `agentCapabilities.promptCapabilities` (the prompt-turn
/// surface this provider drives); a missing prompt capability fails closed.
/// Returns the advertised capabilities the adapter consumes.
fn verify_initialize(message: &Value) -> Result<AgentCapabilities, ExecuteError> {
    let result = message
        .get("result")
        .ok_or(ExecuteError::ProtocolViolation)?;
    if result.get("protocolVersion").and_then(Value::as_i64) != Some(ACP_PROTOCOL_VERSION) {
        return Err(ExecuteError::ProtocolViolation);
    }
    // authMethods must be present and empty: ACP `authenticate` is immediate
    // success and is never treated as a security boundary.
    match result.get("authMethods").and_then(Value::as_array) {
        Some(methods) if methods.is_empty() => {}
        _ => return Err(ExecuteError::ProtocolViolation),
    }
    let capabilities = result
        .get("agentCapabilities")
        .ok_or(ExecuteError::ProtocolViolation)?;
    // The agent must advertise the prompt capability the adapter drives; its
    // absence means this is not a promptable ACP agent and we fail closed.
    if capabilities.get("promptCapabilities").is_none() {
        return Err(ExecuteError::ProtocolViolation);
    }
    // `sessionCapabilities.close` is optional; teardown only sends a graceful
    // `session/close` when it is advertised, and always falls back to the
    // EOF → TERM → KILL ladder.
    let close = capabilities
        .get("sessionCapabilities")
        .and_then(|caps| caps.get("close"))
        .is_some_and(Value::is_object);
    Ok(AgentCapabilities { close })
}

fn extract_session_id(message: &Value) -> Result<String, ExecuteError> {
    message
        .get("result")
        .and_then(|result| result.get("sessionId"))
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .ok_or(ExecuteError::ProtocolViolation)
}

fn message_id(message: &Value) -> Option<i64> {
    message.get("id").and_then(Value::as_i64)
}

/// Map an ACP terminal `stopReason` to a bounded outcome. `end_turn` is the
/// only clean completion; `cancelled` maps to Cancelled; every other reason is
/// a typed failure and never reported as success.
fn finalize_stop_reason(stop: &str, output: String) -> Result<ExecuteOutcome, ExecuteError> {
    match stop {
        "end_turn" => Ok(ExecuteOutcome::Succeeded {
            output,
            usage: UsageInfo {
                prompt_tokens: 0,
                completion_tokens: 0,
                reasoning_tokens: None,
            },
        }),
        "cancelled" => Ok(ExecuteOutcome::Cancelled),
        // max_tokens / refusal / max_turn_requests / unknown all fail closed.
        _ => Err(ExecuteError::ExecutionFailed),
    }
}

/// Classify a JSON-RPC `error` object into a safe category. The upstream
/// message text is never propagated.
fn classify_rpc_error(_error: &Value) -> ExecuteError {
    ExecuteError::ProtocolViolation
}

#[derive(Debug, Clone, Copy)]
enum TeardownSignal {
    Term,
    Kill,
}

/// Signal a whole process group by its stored pgid. Using the pgid captured at
/// spawn (rather than the live child handle) lets teardown reap grandchildren
/// even after the direct child has already exited. On non-unix this is a no-op
/// and the caller's `start_kill`/`kill_on_drop` handles teardown.
fn signal_group(pgid: Option<i32>, signal: TeardownSignal) {
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, killpg};
        use nix::unistd::Pid;
        let Some(pid) = pgid else {
            return;
        };
        let nix_signal = match signal {
            TeardownSignal::Term => Signal::SIGTERM,
            TeardownSignal::Kill => Signal::SIGKILL,
        };
        // The child was spawned into its own process group (pgid == pid).
        let _ = killpg(Pid::from_raw(pid), nix_signal);
    }
    #[cfg(not(unix))]
    {
        let _ = (pgid, signal);
    }
}

/// Wait up to `grace` for the child to exit. Returns true if it exited.
async fn wait_exit(child: &mut Child, grace: Duration) -> bool {
    matches!(timeout(grace, child.wait()).await, Ok(Ok(_)))
}

/// Serialize the pinned MiniMax model config `currentValue` exactly as the
/// official DSH model control does: `JSON.stringify([provider, model])`.
///
/// The runtime verifies an observed `session/set_config_option` state (or the
/// `session/new` config-option snapshot) advertises this exact value before
/// publishing the provider route. It is exported so the config owner and tests
/// can assert the exact wire literal `["minimax","MiniMax-M3"]`.
pub fn expected_model_current_value() -> String {
    // Use serde_json to guarantee byte-exact array encoding.
    Value::Array(vec![
        Value::String(ACP_ROUTE_PROVIDER.to_owned()),
        Value::String(ACP_ROUTE_MODEL.to_owned()),
    ])
    .to_string()
}

/// Verify a model config-option state advertises the exact pinned route as its
/// `currentValue`. Fails closed on any drift.
pub fn verify_model_route(options: &Value) -> Result<(), ExecuteError> {
    let expected = expected_model_current_value();
    let array = options.as_array().ok_or(ExecuteError::ProtocolViolation)?;
    let model_option = array
        .iter()
        .find(|option| option.get("id").and_then(Value::as_str) == Some("model"))
        .ok_or(ExecuteError::ProtocolViolation)?;
    let current = model_option
        .get("currentValue")
        .and_then(Value::as_str)
        .ok_or(ExecuteError::ProtocolViolation)?;
    if current == expected {
        Ok(())
    } else {
        Err(ExecuteError::ProtocolViolation)
    }
}

/// A neutral, redacted description of the resolved child environment for audit
/// and doctor evidence: only the allowed variable *names* appear, never values.
pub fn redacted_child_env_names() -> BTreeMap<&'static str, &'static str> {
    let mut names = BTreeMap::new();
    names.insert(ENV_PATH, "explicit");
    names.insert(ENV_DSH_HOME, "isolated");
    names.insert(ENV_MINIMAX_API_KEY, "[REDACTED]");
    names
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn limits() -> AcpLimits {
        AcpLimits {
            max_line_bytes: 64 * 1024,
            max_output_bytes: 256 * 1024,
            max_update_frames: 1024,
            max_stderr_bytes: 8 * 1024,
            handshake_deadline: Duration::from_secs(5),
            prompt_deadline: Duration::from_secs(30),
            eof_grace: Duration::from_millis(200),
            term_grace: Duration::from_millis(200),
        }
    }

    fn input<'a>(api_key: &'a str) -> AcpConfigInput<'a> {
        AcpConfigInput {
            id: ProviderId("deepseek-harness-acp".to_owned()),
            capability: "llm-chat",
            generation: 1,
            command: "/usr/bin/dsh",
            args: ACP_TEST_ARGS.get_or_init(|| {
                vec![
                    "--profile".to_owned(),
                    "acp".to_owned(),
                    "--patch".to_owned(),
                    "/etc/dxbot/dsh-acp.patch.yaml".to_owned(),
                ]
            }),
            patch_path: "/etc/dxbot/dsh-acp.patch.yaml",
            dsh_home: "/var/lib/dxbot/dsh-home",
            workspace: "/var/lib/dxbot/workspace",
            path_env: "/usr/bin:/bin",
            permission: PermissionPolicy::Reject,
            limits: limits(),
            source_commit: ACP_SOURCE_COMMIT,
            source_version: ACP_SOURCE_VERSION,
            command_sha256: TEST_SHA256,
            patch_sha256: TEST_SHA256,
            audited_patch_sha256: TEST_SHA256,
            api_key,
        }
    }

    static ACP_TEST_ARGS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    const TEST_SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn expected_model_value_is_exact_pinned_route() {
        assert_eq!(
            expected_model_current_value(),
            r#"["minimax","MiniMax-M3"]"#
        );
    }

    #[test]
    fn config_debug_never_contains_secret() {
        let config = AcpProviderConfig::new(input("secret-canary-must-not-appear")).unwrap();
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("secret-canary-must-not-appear"));
        assert!(rendered.contains("[REDACTED]"));
    }

    #[test]
    fn relative_command_fails_closed() {
        let mut bad = input("k");
        bad.command = "dsh";
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidPath)
        );
    }

    #[test]
    fn traversal_path_fails_closed() {
        let mut bad = input("k");
        bad.dsh_home = "/var/lib/../etc/dsh";
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidPath)
        );
    }

    #[test]
    fn empty_credential_fails_closed() {
        assert_eq!(
            AcpProviderConfig::new(input("")).err(),
            Some(AcpConfigError::InvalidCredential)
        );
    }

    #[test]
    fn newline_credential_fails_closed() {
        assert_eq!(
            AcpProviderConfig::new(input("line\nbreak")).err(),
            Some(AcpConfigError::InvalidCredential)
        );
    }

    #[test]
    fn zero_limit_fails_closed() {
        let mut bad = input("k");
        bad.limits.max_line_bytes = 0;
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidLimits)
        );
    }

    #[test]
    fn wrong_source_commit_fails_closed() {
        let mut bad = input("k");
        bad.source_commit = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::SourceCommitMismatch)
        );
    }

    #[test]
    fn wrong_source_version_fails_closed() {
        let mut bad = input("k");
        bad.source_version = "0.9.9";
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::SourceVersionMismatch)
        );
    }

    #[test]
    fn exact_source_identity_is_accepted() {
        let config = AcpProviderConfig::new(input("k")).unwrap();
        assert_eq!(config.source_commit(), ACP_SOURCE_COMMIT);
        assert_eq!(config.source_version(), ACP_SOURCE_VERSION);
    }

    #[test]
    fn argv_must_be_exactly_profile_acp_patch() {
        // Omission.
        let short = vec!["--profile".to_owned(), "acp".to_owned()];
        let mut bad = input("k");
        bad.args = &short;
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidArgv)
        );

        // Extra trailing argument.
        let extra = vec![
            "--profile".to_owned(),
            "acp".to_owned(),
            "--patch".to_owned(),
            "/etc/dxbot/dsh-acp.patch.yaml".to_owned(),
            "--extra".to_owned(),
        ];
        let mut bad = input("k");
        bad.args = &extra;
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidArgv)
        );

        // Duplicate profile flag.
        let dup = vec![
            "--profile".to_owned(),
            "acp".to_owned(),
            "--profile".to_owned(),
            "acp".to_owned(),
        ];
        let mut bad = input("k");
        bad.args = &dup;
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidArgv)
        );

        // Patch path in argv that does not match the validated patch_path.
        let mismatch = vec![
            "--profile".to_owned(),
            "acp".to_owned(),
            "--patch".to_owned(),
            "/etc/dxbot/other.yaml".to_owned(),
        ];
        let mut bad = input("k");
        bad.args = &mismatch;
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidArgv)
        );

        // Reordered flags.
        let reorder = vec![
            "--patch".to_owned(),
            "/etc/dxbot/dsh-acp.patch.yaml".to_owned(),
            "--profile".to_owned(),
            "acp".to_owned(),
        ];
        let mut bad = input("k");
        bad.args = &reorder;
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidArgv)
        );
    }

    #[test]
    fn malformed_digest_fails_closed() {
        // Uppercase hex is rejected (must be lowercase).
        let mut bad = input("k");
        bad.patch_sha256 = "0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef";
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidDigest)
        );

        // Wrong length.
        let mut bad = input("k");
        bad.command_sha256 = "abcd";
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidDigest)
        );

        // A malformed canonical audited digest is rejected too.
        let mut bad = input("k");
        bad.audited_patch_sha256 = "not-hex";
        assert_eq!(
            AcpProviderConfig::new(bad).err(),
            Some(AcpConfigError::InvalidDigest)
        );
    }

    #[test]
    fn config_debug_never_contains_paths_or_digests() {
        let config = AcpProviderConfig::new(input("k")).unwrap();
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("dsh-acp.patch.yaml"));
        assert!(!rendered.contains(TEST_SHA256));
        assert!(!rendered.contains(ACP_SOURCE_COMMIT));
    }

    #[test]
    fn verify_initialize_requires_exact_version_and_empty_auth() {
        let good = json!({
            "id": 1,
            "result": {
                "protocolVersion": 1,
                "authMethods": [],
                "agentCapabilities": {
                    "promptCapabilities": { "image": false },
                    "sessionCapabilities": { "close": {} }
                }
            }
        });
        let caps = verify_initialize(&good).unwrap();
        assert!(caps.close, "close capability must be detected");

        // Prompt capability present but no close capability advertised.
        let no_close = json!({
            "result": {
                "protocolVersion": 1,
                "authMethods": [],
                "agentCapabilities": { "promptCapabilities": {} }
            }
        });
        assert_eq!(
            verify_initialize(&no_close).map(|caps| caps.close),
            Ok(false)
        );

        // ACP models unsupported close as null/omitted and supported close as
        // a SessionCloseCapabilities object. Presence of an explicit null must
        // not trigger an unadvertised session/close request during teardown.
        let null_close = json!({
            "result": {
                "protocolVersion": 1,
                "authMethods": [],
                "agentCapabilities": {
                    "promptCapabilities": {},
                    "sessionCapabilities": { "close": null }
                }
            }
        });
        assert_eq!(
            verify_initialize(&null_close).map(|caps| caps.close),
            Ok(false)
        );

        // Missing the prompt capability the adapter relies on: fail closed.
        let no_prompt = json!({
            "result": {
                "protocolVersion": 1,
                "authMethods": [],
                "agentCapabilities": {}
            }
        });
        assert_eq!(
            verify_initialize(&no_prompt),
            Err(ExecuteError::ProtocolViolation)
        );

        let wrong_version = json!({
            "result": {
                "protocolVersion": 2,
                "authMethods": [],
                "agentCapabilities": { "promptCapabilities": {} }
            }
        });
        assert_eq!(
            verify_initialize(&wrong_version),
            Err(ExecuteError::ProtocolViolation)
        );

        let nonempty_auth = json!({
            "result": {
                "protocolVersion": 1,
                "authMethods": [{ "id": "token" }],
                "agentCapabilities": { "promptCapabilities": {} }
            }
        });
        assert_eq!(
            verify_initialize(&nonempty_auth),
            Err(ExecuteError::ProtocolViolation)
        );
    }

    #[test]
    fn verify_model_route_detects_drift() {
        let good = json!([
            { "id": "model", "currentValue": r#"["minimax","MiniMax-M3"]"# }
        ]);
        assert!(verify_model_route(&good).is_ok());

        let drift = json!([
            { "id": "model", "currentValue": r#"["deepseek-official","deepseek-v4-pro"]"# }
        ]);
        assert_eq!(
            verify_model_route(&drift),
            Err(ExecuteError::ProtocolViolation)
        );
    }

    #[test]
    fn stop_reasons_map_to_typed_outcomes() {
        assert!(matches!(
            finalize_stop_reason("end_turn", "hi".to_owned()),
            Ok(ExecuteOutcome::Succeeded { .. })
        ));
        assert!(matches!(
            finalize_stop_reason("cancelled", String::new()),
            Ok(ExecuteOutcome::Cancelled)
        ));
        for reason in ["max_tokens", "refusal", "max_turn_requests", "surprise"] {
            assert_eq!(
                finalize_stop_reason(reason, String::new()),
                Err(ExecuteError::ExecutionFailed)
            );
        }
    }

    #[test]
    fn redacted_env_names_never_include_values() {
        let names = redacted_child_env_names();
        assert_eq!(names.get("MINIMAX_API_KEY"), Some(&"[REDACTED]"));
        assert_eq!(names.get("DSH_HOME"), Some(&"isolated"));
    }

    #[cfg(unix)]
    mod spawn_assets {
        use super::*;
        use std::os::unix::fs::PermissionsExt;
        use std::time::{SystemTime, UNIX_EPOCH};

        struct Barrier(std::path::PathBuf);

        impl Barrier {
            /// A `0700` root owned by the effective uid, i.e. a trusted barrier
            /// even under the world-writable sticky `/tmp` (the mkdtemp shape).
            fn new(tag: &str) -> Self {
                let suffix = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                let path = std::env::temp_dir().join(format!("dxb-spawn-assets-{tag}-{suffix}"));
                std::fs::create_dir_all(&path).unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
                Self(path)
            }
        }

        impl Drop for Barrier {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        /// Build a config whose command/patch digests match the on-disk files
        /// under `root`, with `root` acting as the trusted barrier.
        fn config_for(root: &std::path::Path) -> AcpProviderConfig {
            let command = root.join("dsh");
            std::fs::write(&command, b"#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o700)).unwrap();
            let patch = root.join("dsh-acp.patch.yaml");
            std::fs::write(&patch, b"# audited patch\n").unwrap();
            std::fs::set_permissions(&patch, std::fs::Permissions::from_mode(0o600)).unwrap();
            let command_sha = stream_sha256_hex(&command, MAX_SPAWN_COMMAND_BYTES).unwrap();
            let patch_sha = stream_sha256_hex(&patch, MAX_SPAWN_PATCH_BYTES).unwrap();
            let patch_str = patch.to_str().unwrap().to_owned();
            let args = vec![
                "--profile".to_owned(),
                "acp".to_owned(),
                "--patch".to_owned(),
                patch_str.clone(),
            ];
            AcpProviderConfig::new(AcpConfigInput {
                id: ProviderId("deepseek-harness-acp".to_owned()),
                capability: "llm-chat",
                generation: 1,
                command: command.to_str().unwrap(),
                args: &args,
                patch_path: &patch_str,
                dsh_home: root.to_str().unwrap(),
                workspace: root.to_str().unwrap(),
                path_env: "/usr/bin:/bin",
                permission: PermissionPolicy::Reject,
                limits: limits(),
                source_commit: ACP_SOURCE_COMMIT,
                source_version: ACP_SOURCE_VERSION,
                command_sha256: &command_sha,
                patch_sha256: &patch_sha,
                audited_patch_sha256: &patch_sha,
                api_key: "k",
            })
            .unwrap()
        }

        #[test]
        fn valid_layout_under_0700_barrier_passes() {
            let root = Barrier::new("ok");
            let config = config_for(&root.0);
            assert!(config.verify_spawn_assets().is_ok());
        }

        #[test]
        fn mutated_patch_fails_closed() {
            let root = Barrier::new("mutate");
            let config = config_for(&root.0);
            // Overwrite the patch after construction: digest no longer matches.
            std::fs::write(config.patch_file(), b"# tampered\n").unwrap();
            std::fs::set_permissions(config.patch_file(), std::fs::Permissions::from_mode(0o600))
                .unwrap();
            assert_eq!(
                config.verify_spawn_assets(),
                Err(ExecuteError::ExecutionFailed)
            );
        }

        #[test]
        fn group_writable_command_fails_closed() {
            let root = Barrier::new("gwrite");
            let config = config_for(&root.0);
            std::fs::set_permissions(
                config.command_path(),
                std::fs::Permissions::from_mode(0o770),
            )
            .unwrap();
            assert_eq!(
                config.verify_spawn_assets(),
                Err(ExecuteError::ExecutionFailed)
            );
        }

        #[test]
        fn symlinked_command_fails_closed() {
            let root = Barrier::new("symlink");
            let config = config_for(&root.0);
            let real = root.0.join("real-dsh");
            std::fs::rename(config.command_path(), &real).unwrap();
            std::os::unix::fs::symlink(&real, config.command_path()).unwrap();
            assert_eq!(
                config.verify_spawn_assets(),
                Err(ExecuteError::ExecutionFailed)
            );
        }

        #[test]
        fn writable_ancestor_below_barrier_fails_closed() {
            // barrier(0700) / writable(0777) / {dsh, patch}: the writable
            // intermediate is below the barrier and must be rejected.
            let root = Barrier::new("ancestor");
            let writable = root.0.join("writable");
            std::fs::create_dir_all(&writable).unwrap();
            std::fs::set_permissions(&writable, std::fs::Permissions::from_mode(0o777)).unwrap();
            let config = config_for(&writable);
            assert_eq!(
                config.verify_spawn_assets(),
                Err(ExecuteError::ExecutionFailed)
            );
        }
    }
}
