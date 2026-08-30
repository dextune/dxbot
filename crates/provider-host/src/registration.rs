//! Immutable per-registration protocol/transport binding (`DXB-DEL-068` H4).
//!
//! Each provider registration owns its own protocol binding, transport binding,
//! generation, and limits. There is no host-wide mutable protocol singleton.
//! An in-flight call uses the binding captured at admission for its whole life,
//! and an atomic replacement publishes a new generation without mutating the
//! captured binding of an old call.
//!
//! `ProtocolKind` is part of the Common contract and is always present: it is a
//! wire-protocol descriptor, not adapter code, so its variants exist regardless
//! of which adapter slices are compiled in. What varies by feature is whether a
//! factory row can *construct* a registration for a given kind. The HTTP
//! `TransportBinding` is present only with the `http-transport` stack; the ACP
//! slice registers via [`ProviderRegistration::detached`] and owns its own
//! newline-delimited JSON-RPC stdio transport.

#![forbid(unsafe_code)]

use std::sync::Arc;

use crate::execute::ExecuteProvider;
#[cfg(feature = "http-transport")]
use crate::protocol::ChatCompletionProtocol;
#[cfg(feature = "http-transport")]
use crate::transport::HttpTransport;

/// The wire protocol a registration speaks. Distinct protocols are distinct
/// bindings: OpenAI Chat Completions, Anthropic Messages, and ACP v1 stdio
/// never share one mutable slot.
///
/// This enum is intentionally complete regardless of which adapter features are
/// enabled — it is a stable Common descriptor, not adapter code — so there are
/// no impossible `cfg` enum arms. A variant is merely never *selected* when its
/// adapter slice is absent, because no factory row constructs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolKind {
    OpenAiChatCompletions,
    AnthropicMessages,
    /// Official DeepSeek Harness automation surface: ACP v1, newline-delimited
    /// JSON-RPC 2.0 over a managed child process' stdin/stdout (`DXB-DEL-068`
    /// H6). It shares no mutable slot with the HTTP protocols and never routes
    /// through the Common HTTP transport; its registration is `detached`.
    AcpV1Stdio,
}

/// Immutable transport binding owned by a single registration.
///
/// Present only with the `http-transport` stack. Detached providers (the ACP
/// slice, the synthetic canary) never construct one.
#[cfg(feature = "http-transport")]
#[derive(Debug, Clone)]
pub struct TransportBinding {
    protocol: ChatCompletionProtocol,
}

#[cfg(feature = "http-transport")]
impl TransportBinding {
    /// Build a binding for the given wire protocol over a Common-owned transport.
    ///
    /// Only the HTTP wire protocols are constructible here. `AcpV1Stdio` is a
    /// detached protocol and must never reach this constructor; passing it is a
    /// programming error and panics rather than silently fabricating an HTTP
    /// binding.
    pub fn new(
        kind: ProtocolKind,
        transport: HttpTransport,
        max_output_bytes: usize,
        max_output_items: usize,
    ) -> Self {
        let protocol = match kind {
            ProtocolKind::OpenAiChatCompletions => {
                ChatCompletionProtocol::new(transport, max_output_bytes, max_output_items)
            }
            ProtocolKind::AnthropicMessages => {
                ChatCompletionProtocol::new_anthropic(transport, max_output_bytes, max_output_items)
            }
            ProtocolKind::AcpV1Stdio => {
                panic!(
                    "AcpV1Stdio registrations are detached and never build an HTTP TransportBinding"
                )
            }
        };
        Self { protocol }
    }

    pub(crate) fn protocol(&self) -> &ChatCompletionProtocol {
        &self.protocol
    }
}

/// Bounded execution limits owned by a registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistrationLimits {
    pub max_output_bytes: usize,
    pub max_output_items: usize,
}

/// An immutable, atomically-replaceable provider registration.
///
/// The registration binds an [`ExecuteProvider`] implementation to the exact
/// protocol/transport it must use, plus the captured generation and limits.
/// Cloning shares the underlying implementation via `Arc`; the record itself is
/// never mutated after publication.
#[derive(Debug, Clone)]
pub struct ProviderRegistration {
    provider: Arc<dyn ExecuteProvider>,
    protocol_kind: ProtocolKind,
    #[cfg(feature = "http-transport")]
    transport: Option<TransportBinding>,
    limits: RegistrationLimits,
}

impl ProviderRegistration {
    /// Build a registration whose implementation drives an HTTP protocol/transport.
    #[cfg(feature = "http-transport")]
    pub fn new(
        provider: Arc<dyn ExecuteProvider>,
        protocol_kind: ProtocolKind,
        transport: TransportBinding,
        limits: RegistrationLimits,
    ) -> Self {
        Self {
            provider,
            protocol_kind,
            transport: Some(transport),
            limits,
        }
    }

    /// Build a registration whose implementation is self-contained and needs
    /// no host-supplied HTTP transport (the ACP slice, the synthetic canary).
    pub fn detached(
        provider: Arc<dyn ExecuteProvider>,
        protocol_kind: ProtocolKind,
        limits: RegistrationLimits,
    ) -> Self {
        Self {
            provider,
            protocol_kind,
            #[cfg(feature = "http-transport")]
            transport: None,
            limits,
        }
    }

    pub fn provider(&self) -> &Arc<dyn ExecuteProvider> {
        &self.provider
    }

    pub fn protocol_kind(&self) -> ProtocolKind {
        self.protocol_kind
    }

    #[cfg(feature = "http-transport")]
    pub fn transport(&self) -> Option<&TransportBinding> {
        self.transport.as_ref()
    }

    pub fn limits(&self) -> RegistrationLimits {
        self.limits
    }
}
