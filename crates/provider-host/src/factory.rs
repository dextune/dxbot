//! Static `ProviderFactory` inventory (`DXB-DEL-068` H5).
//!
//! A compile-time inventory maps an adapter key to its config schema version,
//! advertised capability, credential mode, and a constructor that produces a
//! complete candidate [`ProviderRegistration`]. The Runtime config owner looks
//! up the registry by key and never hard-codes a concrete `match` over adapter
//! names. Unknown and duplicate keys are typed failures; there is no unknown
//! key fallback, no dynamic ABI, and no reflection.

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

#[cfg(any(
    feature = "direct-deepseek",
    feature = "direct-minimax",
    feature = "dsh-acp"
))]
use dxbot_core::types::ProviderId;

#[cfg(feature = "dsh-acp")]
use crate::acp::{
    ACP_ADAPTER_KEY, AcpConfigInput, AcpLimits, AcpProviderConfig, DeepSeekHarnessAcpProvider,
    PermissionPolicy,
};
#[cfg(feature = "direct-deepseek")]
use crate::deepseek::DeepSeekFlashAdapter;
#[cfg(any(feature = "direct-deepseek", feature = "direct-minimax"))]
use crate::http_provider::HttpExecuteProvider;
#[cfg(feature = "direct-minimax")]
use crate::minimax::MiniMaxM3Adapter;
#[cfg(any(feature = "direct-deepseek", feature = "direct-minimax"))]
use crate::registration::TransportBinding;
use crate::registration::{ProtocolKind, ProviderRegistration, RegistrationLimits};
#[cfg(any(feature = "direct-deepseek", feature = "direct-minimax"))]
use crate::transport::{HttpTransport, TransportBuildError};

#[cfg(any(feature = "http-transport", feature = "dsh-acp"))]
const DEFAULT_MAX_OUTPUT_BYTES: usize = 1024 * 1024;
#[cfg(any(feature = "http-transport", feature = "dsh-acp"))]
const DEFAULT_MAX_OUTPUT_ITEMS: usize = 1000;

/// Credential installation mode a factory requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialMode {
    /// `Authorization: Bearer <secret>`.
    Bearer,
    /// Anthropic `x-api-key: <secret>`.
    AnthropicKey,
    /// The secret is materialized into exactly one scrubbed child env var
    /// (`MINIMAX_API_KEY`) rather than an HTTP header.
    ChildEnv,
}

/// Extra, adapter-shaped inputs a subprocess/ACP factory needs beyond the
/// HTTP-oriented [`FactoryConfig`] fields. HTTP factories ignore this. Present
/// only with the `dsh-acp` slice.
#[cfg(feature = "dsh-acp")]
#[derive(Debug, Clone)]
pub struct AcpFactoryConfig {
    /// Absolute executable path (no shell).
    pub command: String,
    /// Exact argument vector (e.g. `--profile acp --patch <path>`).
    pub args: Vec<String>,
    /// Absolute path to the DXBOT-owned audited profile patch.
    pub patch_path: String,
    /// Absolute, isolated `DSH_HOME`.
    pub dsh_home: String,
    /// Absolute validated workspace / session cwd.
    pub workspace: String,
    /// Explicit `PATH` handed to the child.
    pub path_env: String,
    /// Unattended permission default.
    pub permission: PermissionPolicy,
    /// Positive-finite resource bounds.
    pub limits: AcpLimits,
    /// Pinned upstream source commit; must equal `ACP_SOURCE_COMMIT`.
    pub source_commit: String,
    /// Pinned upstream source version; must equal `ACP_SOURCE_VERSION`.
    pub source_version: String,
    /// Expected lowercase 64-hex SHA-256 of the on-disk command executable.
    pub command_sha256: String,
    /// Expected lowercase 64-hex SHA-256 of the on-disk audited patch file.
    pub patch_sha256: String,
    /// The canonical compile-time audited patch digest owned by the Runtime
    /// config owner (`runtime-host`), threaded verbatim into the provider so
    /// the spawn-time recheck compares against the same value composition did.
    pub audited_patch_sha256: String,
}

/// Decoded, validated config a factory needs to build a candidate registration.
///
/// The Runtime config owner decodes the persisted config into this neutral
/// shape; the factory maps it to a concrete adapter + protocol/transport
/// binding. The secret is borrowed for the construction call only and is never
/// retained by the factory or the registration.
#[derive(Debug)]
pub struct FactoryConfig<'a> {
    pub provider_id: &'a str,
    pub capability: &'a str,
    pub generation: i64,
    pub endpoint: &'a str,
    pub model: &'a str,
    pub timeout: Duration,
    pub secret: &'a str,
    /// Present only for the subprocess/ACP factory; HTTP factories leave it
    /// `None` and never read it. The field itself exists only with the
    /// `dsh-acp` slice, so removing the slice removes ACP config decoding.
    #[cfg(feature = "dsh-acp")]
    pub acp: Option<AcpFactoryConfig>,
}

/// Typed failure building a candidate registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactoryError {
    /// The adapter key is not present in the static inventory.
    UnknownAdapter { key: String },
    /// The credential could not be installed into the transport.
    CredentialUnavailable,
    /// The decoded config is invalid for this factory.
    InvalidConfig { reason: String },
}

/// Static descriptor for one production provider factory.
pub struct ProviderFactory {
    /// Stable adapter key used by config and discovery.
    pub key: &'static str,
    /// Supported config schema version.
    pub schema_version: u32,
    /// Advertised capability class.
    pub capability_class: &'static str,
    /// Wire protocol this factory binds.
    pub protocol_kind: ProtocolKind,
    /// Credential mode required.
    pub credential_mode: CredentialMode,
    /// Build a complete candidate registration from decoded config + secret.
    pub build: fn(&FactoryConfig<'_>) -> Result<ProviderRegistration, FactoryError>,
}

impl std::fmt::Debug for ProviderFactory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderFactory")
            .field("key", &self.key)
            .field("schema_version", &self.schema_version)
            .field("capability_class", &self.capability_class)
            .field("protocol_kind", &self.protocol_kind)
            .field("credential_mode", &self.credential_mode)
            .finish_non_exhaustive()
    }
}

/// The compile-time inventory of production provider factories.
///
/// Adding a provider means adding one row here plus its adapter module and
/// conformance instantiation; removing a provider means deleting its slice
/// (feature off) without touching unrelated rows. Each row is gated by its
/// adapter feature, so a removed adapter contributes no key, no schema, and no
/// constructor — an unknown/removed key then fails closed in
/// [`lookup_factory`].
static PROVIDER_FACTORIES: &[ProviderFactory] = &[
    #[cfg(feature = "direct-deepseek")]
    ProviderFactory {
        key: "deepseek-flash",
        schema_version: 1,
        capability_class: "llm-chat",
        protocol_kind: ProtocolKind::OpenAiChatCompletions,
        credential_mode: CredentialMode::Bearer,
        build: build_deepseek_flash,
    },
    #[cfg(feature = "direct-minimax")]
    ProviderFactory {
        key: "minimax-m3",
        schema_version: 1,
        capability_class: "llm-chat",
        protocol_kind: ProtocolKind::AnthropicMessages,
        credential_mode: CredentialMode::AnthropicKey,
        build: build_minimax_m3,
    },
    #[cfg(feature = "dsh-acp")]
    ProviderFactory {
        key: ACP_ADAPTER_KEY,
        schema_version: 2,
        capability_class: "llm-chat",
        protocol_kind: ProtocolKind::AcpV1Stdio,
        credential_mode: CredentialMode::ChildEnv,
        build: build_deepseek_harness_acp,
    },
];

/// The known adapter keys, in inventory order. Used for diagnostics and
/// duplicate detection.
pub fn factory_keys() -> Vec<&'static str> {
    PROVIDER_FACTORIES
        .iter()
        .map(|factory| factory.key)
        .collect()
}

/// Look up a factory by exact adapter key. Unknown keys fail closed.
pub fn lookup_factory(key: &str) -> Result<&'static ProviderFactory, FactoryError> {
    // A duplicate key in the static inventory is a programming error; assert it
    // fails closed rather than silently selecting the first row.
    let mut matches = PROVIDER_FACTORIES
        .iter()
        .filter(|factory| factory.key == key);
    let first = matches.next().ok_or_else(|| FactoryError::UnknownAdapter {
        key: key.to_owned(),
    })?;
    if matches.next().is_some() {
        return Err(FactoryError::InvalidConfig {
            reason: format!("duplicate factory key in static inventory: {key}"),
        });
    }
    Ok(first)
}

#[cfg(any(feature = "http-transport", feature = "dsh-acp"))]
fn default_limits() -> RegistrationLimits {
    RegistrationLimits {
        max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
        max_output_items: DEFAULT_MAX_OUTPUT_ITEMS,
    }
}

#[cfg(feature = "direct-deepseek")]
fn build_deepseek_flash(config: &FactoryConfig<'_>) -> Result<ProviderRegistration, FactoryError> {
    let limits = default_limits();
    let transport = HttpTransport::new(config.endpoint, config.timeout)
        .with_bearer(config.secret)
        .map_err(map_transport_error)?;
    let binding = TransportBinding::new(
        ProtocolKind::OpenAiChatCompletions,
        transport,
        limits.max_output_bytes,
        limits.max_output_items,
    );
    let adapter = Box::new(DeepSeekFlashAdapter::configured(
        ProviderId(config.provider_id.to_owned()),
        config.capability,
        config.generation,
        config.model,
    ));
    let provider = HttpExecuteProvider::new(
        adapter,
        &binding,
        limits.max_output_bytes,
        limits.max_output_items,
    );
    Ok(ProviderRegistration::new(
        provider as Arc<_>,
        ProtocolKind::OpenAiChatCompletions,
        binding,
        limits,
    ))
}

#[cfg(feature = "direct-minimax")]
fn build_minimax_m3(config: &FactoryConfig<'_>) -> Result<ProviderRegistration, FactoryError> {
    let limits = default_limits();
    let transport = HttpTransport::new(config.endpoint, config.timeout)
        .with_anthropic_key(config.secret)
        .map_err(map_transport_error)?;
    let binding = TransportBinding::new(
        ProtocolKind::AnthropicMessages,
        transport,
        limits.max_output_bytes,
        limits.max_output_items,
    );
    let adapter = Box::new(MiniMaxM3Adapter::configured(
        ProviderId(config.provider_id.to_owned()),
        config.capability,
        config.generation,
        config.model,
    ));
    let provider = HttpExecuteProvider::new(
        adapter,
        &binding,
        limits.max_output_bytes,
        limits.max_output_items,
    );
    Ok(ProviderRegistration::new(
        provider as Arc<_>,
        ProtocolKind::AnthropicMessages,
        binding,
        limits,
    ))
}

#[cfg(feature = "dsh-acp")]
fn build_deepseek_harness_acp(
    config: &FactoryConfig<'_>,
) -> Result<ProviderRegistration, FactoryError> {
    let acp = config
        .acp
        .as_ref()
        .ok_or_else(|| FactoryError::InvalidConfig {
            reason: "deepseek-harness-acp requires harness config".to_owned(),
        })?;
    let provider_config = AcpProviderConfig::new(AcpConfigInput {
        id: ProviderId(config.provider_id.to_owned()),
        capability: config.capability,
        generation: config.generation,
        command: &acp.command,
        args: &acp.args,
        patch_path: &acp.patch_path,
        dsh_home: &acp.dsh_home,
        workspace: &acp.workspace,
        path_env: &acp.path_env,
        permission: acp.permission,
        limits: acp.limits,
        source_commit: &acp.source_commit,
        source_version: &acp.source_version,
        command_sha256: &acp.command_sha256,
        patch_sha256: &acp.patch_sha256,
        audited_patch_sha256: &acp.audited_patch_sha256,
        api_key: config.secret,
    })
    .map_err(|error| FactoryError::InvalidConfig {
        reason: format!("invalid harness config: {error:?}"),
    })?;
    let provider = Arc::new(DeepSeekHarnessAcpProvider::new(provider_config));
    // ACP is a self-contained subprocess protocol: no HTTP transport binding.
    Ok(ProviderRegistration::detached(
        provider as Arc<_>,
        ProtocolKind::AcpV1Stdio,
        default_limits(),
    ))
}

#[cfg(any(feature = "direct-deepseek", feature = "direct-minimax"))]
fn map_transport_error(error: TransportBuildError) -> FactoryError {
    match error {
        TransportBuildError::InvalidCredential | TransportBuildError::ClientBuild => {
            FactoryError::CredentialUnavailable
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn known_keys_are_unique_and_resolvable() {
        let keys = factory_keys();
        // Every advertised key resolves and is unique; which keys are present
        // depends on which adapter slices are compiled in.
        #[cfg(feature = "direct-deepseek")]
        assert!(keys.contains(&"deepseek-flash"));
        #[cfg(feature = "direct-minimax")]
        assert!(keys.contains(&"minimax-m3"));
        #[cfg(feature = "dsh-acp")]
        assert!(keys.contains(&ACP_ADAPTER_KEY));
        for key in &keys {
            assert!(lookup_factory(key).is_ok(), "key {key} must resolve");
        }
    }

    #[test]
    fn unknown_key_fails_closed() {
        assert_eq!(
            lookup_factory("no-such-adapter").err(),
            Some(FactoryError::UnknownAdapter {
                key: "no-such-adapter".to_owned(),
            })
        );
    }

    /// A removed adapter's key is unknown and fails closed exactly like any
    /// other unknown key: there is no dead row and no fallback. When a `direct-*`
    /// slice is absent its key must not resolve.
    #[cfg(not(feature = "direct-deepseek"))]
    #[test]
    fn removed_deepseek_key_fails_closed() {
        assert!(matches!(
            lookup_factory("deepseek-flash"),
            Err(FactoryError::UnknownAdapter { .. })
        ));
    }

    #[cfg(not(feature = "direct-minimax"))]
    #[test]
    fn removed_minimax_key_fails_closed() {
        assert!(matches!(
            lookup_factory("minimax-m3"),
            Err(FactoryError::UnknownAdapter { .. })
        ));
    }

    #[cfg(not(feature = "dsh-acp"))]
    #[test]
    fn removed_acp_key_fails_closed() {
        assert!(matches!(
            lookup_factory("deepseek-harness-acp"),
            Err(FactoryError::UnknownAdapter { .. })
        ));
    }

    #[cfg(feature = "direct-minimax")]
    #[test]
    fn factory_builds_registration_without_retaining_secret() {
        let factory = lookup_factory("minimax-m3").expect("factory");
        assert_eq!(factory.schema_version, 1);
        assert_eq!(factory.capability_class, "llm-chat");
        let config = FactoryConfig {
            provider_id: "minimax-m3-production",
            capability: "llm-chat",
            generation: 3,
            endpoint: "https://api.minimax.io/anthropic/v1",
            model: "MiniMax-M3",
            timeout: Duration::from_millis(30_000),
            secret: "secret-canary-must-not-appear",
            #[cfg(feature = "dsh-acp")]
            acp: None,
        };
        let registration = (factory.build)(&config).expect("registration");
        assert_eq!(
            registration.provider().id(),
            &ProviderId("minimax-m3-production".to_owned())
        );
        assert_eq!(registration.provider().generation(), 3);
        let rendered = format!("{registration:?}");
        assert!(!rendered.contains("secret-canary-must-not-appear"));
    }

    #[cfg(feature = "dsh-acp")]
    const TEST_SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[cfg(feature = "dsh-acp")]
    fn acp_limits() -> AcpLimits {
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

    #[cfg(feature = "dsh-acp")]
    #[test]
    fn acp_factory_row_is_schema_v2_stdio_child_env() {
        let factory = lookup_factory("deepseek-harness-acp").expect("acp factory");
        assert_eq!(factory.schema_version, 2);
        assert_eq!(factory.protocol_kind, ProtocolKind::AcpV1Stdio);
        assert_eq!(factory.credential_mode, CredentialMode::ChildEnv);
    }

    #[cfg(feature = "dsh-acp")]
    #[test]
    fn acp_factory_builds_detached_registration_without_retaining_secret() {
        let factory = lookup_factory("deepseek-harness-acp").expect("acp factory");
        let config = FactoryConfig {
            provider_id: "deepseek-harness-acp",
            capability: "llm-chat",
            generation: 4,
            endpoint: "",
            model: "MiniMax-M3",
            timeout: Duration::from_millis(30_000),
            secret: "acp-secret-canary-must-not-appear",
            acp: Some(AcpFactoryConfig {
                command: "/usr/bin/dsh".to_owned(),
                args: vec![
                    "--profile".to_owned(),
                    "acp".to_owned(),
                    "--patch".to_owned(),
                    "/etc/dxbot/dsh-acp.patch.yaml".to_owned(),
                ],
                patch_path: "/etc/dxbot/dsh-acp.patch.yaml".to_owned(),
                dsh_home: "/var/lib/dxbot/dsh-home".to_owned(),
                workspace: "/var/lib/dxbot/workspace".to_owned(),
                path_env: "/usr/bin:/bin".to_owned(),
                permission: PermissionPolicy::Reject,
                limits: acp_limits(),
                source_commit: crate::acp::ACP_SOURCE_COMMIT.to_owned(),
                source_version: crate::acp::ACP_SOURCE_VERSION.to_owned(),
                command_sha256: TEST_SHA256.to_owned(),
                patch_sha256: TEST_SHA256.to_owned(),
                audited_patch_sha256: TEST_SHA256.to_owned(),
            }),
        };
        let registration = (factory.build)(&config).expect("acp registration");
        assert_eq!(registration.protocol_kind(), ProtocolKind::AcpV1Stdio);
        #[cfg(feature = "http-transport")]
        assert!(registration.transport().is_none());
        assert_eq!(registration.provider().generation(), 4);
        let rendered = format!("{registration:?}");
        assert!(!rendered.contains("acp-secret-canary-must-not-appear"));
    }

    #[cfg(feature = "dsh-acp")]
    #[test]
    fn acp_factory_without_harness_config_fails_closed() {
        let factory = lookup_factory("deepseek-harness-acp").expect("acp factory");
        let config = FactoryConfig {
            provider_id: "deepseek-harness-acp",
            capability: "llm-chat",
            generation: 4,
            endpoint: "",
            model: "MiniMax-M3",
            timeout: Duration::from_millis(30_000),
            secret: "k",
            acp: None,
        };
        assert!(matches!(
            (factory.build)(&config),
            Err(FactoryError::InvalidConfig { .. })
        ));
    }
}
