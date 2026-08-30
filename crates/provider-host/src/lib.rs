#![forbid(unsafe_code)]

// Common contract (always present): Execute SPI, factory registry, host,
// registration, model-adapter SPI, and the always-available cancellation/usage
// primitives. These are never gated by a production adapter feature so the
// Common framework owns correctness regardless of which adapter slices are
// compiled in.
pub mod cancellation;
pub mod execute;
pub mod factory;
pub mod harness;
pub mod registration;

// The model-adapter SPI (`RealProvider`) is only meaningful with the HTTP
// transport stack, since it supplies model-specific request/event shapes to the
// Common HTTP protocol handler. It is present only with `http-transport`.
#[cfg(feature = "http-transport")]
pub mod real_provider;

// The Common HTTP transport/protocol stack, shared by the direct HTTP adapters
// and the HTTP conformance reference. Present only when at least one HTTP slice
// is enabled (`http-transport`, enabled transitively by `direct-*` or
// `testkit`).
#[cfg(feature = "http-transport")]
pub mod http_provider;
#[cfg(feature = "http-transport")]
pub mod protocol;
#[cfg(feature = "http-transport")]
pub mod transport;

// Production adapter slices. Each module compiles only when its feature is on.
#[cfg(feature = "dsh-acp")]
pub mod acp;
#[cfg(feature = "direct-deepseek")]
pub mod deepseek;
#[cfg(feature = "direct-minimax")]
pub mod minimax;

#[cfg(feature = "testkit")]
pub mod reference_adapter;

#[cfg(feature = "testkit")]
pub mod testkit;

// ---- Common re-exports (always present) ----
pub use execute::{
    AllowOnceGrant, ExecuteError, ExecuteEvent, ExecuteFuture, ExecuteOutcome, ExecuteProvider,
    ExecuteRequest, ExecuteResult, ToolDisposition, ToolEffectRecord,
};
pub use factory::{
    CredentialMode, FactoryConfig, FactoryError, ProviderFactory, factory_keys, lookup_factory,
};
pub use harness::{
    Evidence, HarnessError, ProviderActivityRef, ProviderHost, ProviderInfo, ProviderStatus,
    TaskDescription, TaskResult, TaskStatus,
};
pub use registration::{ProtocolKind, ProviderRegistration, RegistrationLimits};

// ---- HTTP transport re-exports ----
#[cfg(feature = "http-transport")]
pub use http_provider::HttpExecuteProvider;
#[cfg(feature = "http-transport")]
pub use protocol::{
    ChatCompletionProtocol, ChatMessage, ProviderError, ProviderEvent, ProviderExecuteConfig,
    ProviderRequest,
};
#[cfg(feature = "http-transport")]
pub use real_provider::RealProvider;
#[cfg(feature = "http-transport")]
pub use registration::TransportBinding;
#[cfg(feature = "http-transport")]
pub use transport::{HttpTransport, TransportBuildError};

// The Execute contract's cancellation token / usage type are always available
// (the ACP slice and the Common host use them even without the HTTP stack).
// They live in the always-present `cancellation` module; the HTTP `protocol`
// module re-uses the same types so there is a single canonical owner.
pub use cancellation::{CancellationToken, UsageInfo};

// ---- Direct DeepSeek slice re-exports ----
#[cfg(feature = "direct-deepseek")]
pub use deepseek::DeepSeekFlashAdapter;

// ---- Direct MiniMax slice re-exports ----
#[cfg(feature = "direct-minimax")]
pub use minimax::MiniMaxM3Adapter;

// ---- DSH ACP slice re-exports ----
#[cfg(feature = "dsh-acp")]
pub use acp::{
    ACP_ADAPTER_KEY, ACP_ARG_PATCH_FLAG, ACP_ARG_PROFILE_FLAG, ACP_ARG_PROFILE_VALUE,
    ACP_PROTOCOL_VERSION, ACP_ROUTE_MODEL, ACP_ROUTE_PROVIDER, ACP_SOURCE_COMMIT,
    ACP_SOURCE_VERSION, AcpConfigError, AcpConfigInput, AcpLimits, AcpProviderConfig,
    DeepSeekHarnessAcpProvider, PermissionPolicy, expected_model_current_value,
    redacted_child_env_names, verify_model_route,
};
#[cfg(feature = "dsh-acp")]
pub use factory::AcpFactoryConfig;

// ---- Testkit re-exports ----
#[cfg(feature = "testkit")]
pub use reference_adapter::{
    ConformanceError, ConformanceOutcome, ConformanceRequest, CredentialMaterial,
    HttpReferenceAdapter, NormalizedUsage, ReferenceAdapter, SubprocessReferenceAdapter,
    UsageSource,
};
#[cfg(feature = "testkit")]
pub use testkit::{
    CanaryMode, ConformanceCapability, ConformanceCase, ConformanceColumn, ConformanceOutcomeKind,
    ConformanceRow, ConformanceSuite, ReferenceProvider, TestCanaryProvider, conformance_row,
    conformance_rows,
};
