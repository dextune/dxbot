//! Adapter-independent provider conformance references.
//!
//! These adapters are executable contract probes, not additional product
//! providers. HTTP reuses the Common-owned protocol/transport. Subprocess uses
//! a deliberately tiny bounded frame protocol to expose different lifecycle
//! and cancellation behavior.

use std::collections::BTreeSet;
use std::fmt;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use runtime_audit::{
    DiagnosticComponent, DiagnosticFamily, DiagnosticReason, DiagnosticSeverity, DiagnosticSink,
    SafeAttribute, SafeAttributeKey,
};

use crate::protocol::{
    CancellationToken, ChatCompletionProtocol, ChatMessage, ProviderError, ProviderEvent,
    ProviderExecuteConfig, ProviderRequest, UsageInfo,
};
use crate::transport::{HttpTransport, TransportBuildError};

const MAX_REFERENCE_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_REFERENCE_OUTPUT_ITEMS: usize = 256;
const SUBPROCESS_POLL_INTERVAL: Duration = Duration::from_millis(10);
/// `ETXTBSY` (text file busy) errno on Linux/Android. A freshly published
/// executable can briefly report this until the writer's close propagates.
const ETXTBSY: i32 = 26;
/// Bounded retries for a transient `ETXTBSY` on spawn before treating it as a
/// hard transport failure.
const MAX_SPAWN_TEXT_BUSY_RETRIES: u32 = 50;
const SPAWN_TEXT_BUSY_BACKOFF: Duration = Duration::from_millis(10);

#[derive(Clone, Copy)]
pub struct CredentialMaterial<'a> {
    reference: &'a str,
    secret: &'a str,
    expires_at: Option<Instant>,
}

impl fmt::Debug for CredentialMaterial<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CredentialMaterial")
            .field("reference", &self.reference)
            .field("secret", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl<'a> CredentialMaterial<'a> {
    pub fn new(reference: &'a str, secret: &'a str) -> Self {
        Self {
            reference,
            secret,
            expires_at: None,
        }
    }

    pub fn with_expiry(mut self, expires_at: Instant) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn reference(&self) -> &str {
        self.reference
    }

    fn secret(&self) -> &str {
        self.secret
    }

    fn validate(&self) -> Result<(), ConformanceError> {
        if self
            .expires_at
            .is_some_and(|expires_at| expires_at <= Instant::now())
        {
            return Err(ConformanceError::CredentialExpired);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ConformanceRequest<'a> {
    pub model: &'a str,
    pub prompt: &'a str,
    pub deadline: Duration,
    pub cancellation: CancellationToken,
    pub credential: Option<CredentialMaterial<'a>>,
    pub required_action_grant: Option<&'a str>,
    pub action_grants: &'a BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageSource {
    Reported,
    Derived,
    Estimated,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizedUsage {
    pub input_units: Option<u64>,
    pub output_units: Option<u64>,
    pub source: UsageSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceOutcome {
    pub content: String,
    pub usage: NormalizedUsage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConformanceError {
    ActionGrantRequired { action: String },
    CredentialExpired,
    CredentialUnavailable,
    Cancelled,
    DeadlineExceeded,
    TransportUnavailable,
    MalformedFrame,
    OutputExceeded,
    ExecutionFailed,
}

pub trait ReferenceAdapter: fmt::Debug {
    fn execute(
        &self,
        request: &ConformanceRequest<'_>,
    ) -> Result<ConformanceOutcome, ConformanceError>;
}

pub struct HttpReferenceAdapter {
    protocol: ChatCompletionProtocol,
    runtime: tokio::runtime::Runtime,
    diagnostics: Option<DiagnosticSink>,
}

impl fmt::Debug for HttpReferenceAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpReferenceAdapter")
            .field("protocol", &self.protocol)
            .finish_non_exhaustive()
    }
}

impl HttpReferenceAdapter {
    pub fn new(transport: HttpTransport) -> Result<Self, ConformanceError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .map_err(|_| ConformanceError::TransportUnavailable)?;
        Ok(Self {
            protocol: ChatCompletionProtocol::new(
                transport,
                MAX_REFERENCE_OUTPUT_BYTES,
                MAX_REFERENCE_OUTPUT_ITEMS,
            ),
            runtime,
            diagnostics: None,
        })
    }

    pub fn with_diagnostics(mut self, diagnostics: DiagnosticSink) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }
}

impl ReferenceAdapter for HttpReferenceAdapter {
    fn execute(
        &self,
        request: &ConformanceRequest<'_>,
    ) -> Result<ConformanceOutcome, ConformanceError> {
        validate_request(request)?;
        if request.cancellation.is_cancelled() {
            self.emit(
                DiagnosticReason::ProviderCancelled,
                DiagnosticSeverity::Info,
            );
            return Err(ConformanceError::Cancelled);
        }

        let protocol = match request.credential {
            Some(credential) => {
                credential.validate()?;
                let transport = self
                    .protocol
                    .transport
                    .with_bearer(credential.secret())
                    .map_err(map_transport_build_error)?;
                ChatCompletionProtocol::new(
                    transport,
                    self.protocol.max_output_bytes,
                    self.protocol.max_output_items,
                )
            }
            None => self.protocol.clone(),
        };

        let provider_request = ProviderRequest {
            model: request.model.to_owned(),
            messages: vec![ChatMessage {
                role: "user".to_owned(),
                content: request.prompt.to_owned(),
            }],
            max_tokens: None,
            temperature: None,
            stream: true,
        };
        let config = ProviderExecuteConfig {
            deadline: Some(tokio::time::Instant::now() + request.deadline),
            cancel_token: Some(request.cancellation.clone()),
            max_output_bytes: MAX_REFERENCE_OUTPUT_BYTES,
            max_output_items: MAX_REFERENCE_OUTPUT_ITEMS,
        };
        let events = self
            .runtime
            .block_on(protocol.execute(&provider_request, &config))
            .map_err(|error| map_provider_error(error, self.diagnostics.as_ref()))?;
        normalize_provider_events(&events)
    }
}

#[derive(Clone)]
pub struct SubprocessReferenceAdapter {
    program: PathBuf,
    args: Vec<String>,
    diagnostics: Option<DiagnosticSink>,
}

impl fmt::Debug for SubprocessReferenceAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubprocessReferenceAdapter")
            .field("program", &self.program)
            .field("arg_count", &self.args.len())
            .finish_non_exhaustive()
    }
}

impl SubprocessReferenceAdapter {
    pub fn new(program: &Path, args: &[String]) -> Self {
        Self {
            program: program.to_path_buf(),
            args: args.to_vec(),
            diagnostics: None,
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: DiagnosticSink) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    fn emit(&self, reason: DiagnosticReason, severity: DiagnosticSeverity) {
        emit_provider_diagnostic(
            self.diagnostics.as_ref(),
            "subprocess-reference",
            reason,
            severity,
        );
    }
}

impl ReferenceAdapter for SubprocessReferenceAdapter {
    fn execute(
        &self,
        request: &ConformanceRequest<'_>,
    ) -> Result<ConformanceOutcome, ConformanceError> {
        validate_request(request)?;
        if request.cancellation.is_cancelled() {
            self.emit(
                DiagnosticReason::ProviderCancelled,
                DiagnosticSeverity::Info,
            );
            return Err(ConformanceError::Cancelled);
        }
        if let Some(credential) = request.credential {
            credential.validate()?;
        }

        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .env_clear()
            .env("DXBOT_REFERENCE_MODEL", request.model)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(credential) = request.credential {
            command.env("DXBOT_REFERENCE_CREDENTIAL", credential.secret());
        }
        // A provider executable that was published very recently can transiently
        // fail `execve` with `ETXTBSY` (text file busy, errno 26 on Linux/Android)
        // while the writer's close propagates. Retry a bounded number of times
        // with a short backoff; any other spawn error is a genuine transport
        // failure and is surfaced immediately.
        let mut child = {
            let mut attempts = 0_u32;
            loop {
                match command.spawn() {
                    Ok(child) => break child,
                    Err(error)
                        if error.raw_os_error() == Some(ETXTBSY)
                            && attempts < MAX_SPAWN_TEXT_BUSY_RETRIES =>
                    {
                        if request.cancellation.is_cancelled() {
                            return Err(ConformanceError::Cancelled);
                        }
                        attempts = attempts.saturating_add(1);
                        thread::sleep(SPAWN_TEXT_BUSY_BACKOFF);
                        continue;
                    }
                    Err(_) => return Err(ConformanceError::TransportUnavailable),
                }
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            if stdin.write_all(request.prompt.as_bytes()).is_err() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ConformanceError::ExecutionFailed);
            }
        } else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ConformanceError::ExecutionFailed);
        }

        let deadline = Instant::now() + request.deadline;
        loop {
            if request.cancellation.is_cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                self.emit(
                    DiagnosticReason::ProviderCancelled,
                    DiagnosticSeverity::Info,
                );
                return Err(ConformanceError::Cancelled);
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ConformanceError::DeadlineExceeded);
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        return Err(ConformanceError::ExecutionFailed);
                    }
                    let Some(stdout) = child.stdout.take() else {
                        return Err(ConformanceError::ExecutionFailed);
                    };
                    let mut bounded = stdout.take(
                        u64::try_from(MAX_REFERENCE_OUTPUT_BYTES)
                            .unwrap_or(u64::MAX)
                            .saturating_add(1),
                    );
                    let mut bytes = Vec::new();
                    bounded
                        .read_to_end(&mut bytes)
                        .map_err(|_| ConformanceError::ExecutionFailed)?;
                    if bytes.len() > MAX_REFERENCE_OUTPUT_BYTES {
                        return Err(ConformanceError::OutputExceeded);
                    }
                    let text = std::str::from_utf8(&bytes).map_err(|_| {
                        self.emit(
                            DiagnosticReason::ProviderMalformedFrame,
                            DiagnosticSeverity::Error,
                        );
                        ConformanceError::MalformedFrame
                    })?;
                    return parse_subprocess_frames(text, request, self.diagnostics.as_ref());
                }
                Ok(None) => thread::sleep(SUBPROCESS_POLL_INTERVAL),
                Err(_) => return Err(ConformanceError::ExecutionFailed),
            }
        }
    }
}

impl HttpReferenceAdapter {
    fn emit(&self, reason: DiagnosticReason, severity: DiagnosticSeverity) {
        emit_provider_diagnostic(
            self.diagnostics.as_ref(),
            "http-reference",
            reason,
            severity,
        );
    }
}

fn validate_request(request: &ConformanceRequest<'_>) -> Result<(), ConformanceError> {
    if request.deadline.is_zero() {
        return Err(ConformanceError::DeadlineExceeded);
    }
    if let Some(credential) = request.credential {
        credential.validate()?;
    }
    if let Some(action) = request.required_action_grant {
        if !request.action_grants.contains(action) {
            return Err(ConformanceError::ActionGrantRequired {
                action: action.to_owned(),
            });
        }
    }
    Ok(())
}

fn normalize_provider_events(
    events: &[ProviderEvent],
) -> Result<ConformanceOutcome, ConformanceError> {
    let mut content = String::new();
    let mut usage = None;
    let mut completed = 0_u8;
    for event in events {
        match event {
            ProviderEvent::ContentDelta { text, .. } => content.push_str(text),
            ProviderEvent::ReasoningDelta { .. } => {}
            ProviderEvent::Completed { usage: reported } => {
                completed = completed.saturating_add(1);
                usage = Some(normalize_usage(*reported));
            }
            ProviderEvent::Failed { .. } => return Err(ConformanceError::ExecutionFailed),
            ProviderEvent::Cancelled => return Err(ConformanceError::Cancelled),
        }
    }
    if completed != 1 {
        return Err(ConformanceError::MalformedFrame);
    }
    Ok(ConformanceOutcome {
        content,
        usage: usage.unwrap_or(NormalizedUsage {
            input_units: None,
            output_units: None,
            source: UsageSource::Unknown,
        }),
    })
}

fn normalize_usage(usage: UsageInfo) -> NormalizedUsage {
    if usage.prompt_tokens == 0 && usage.completion_tokens == 0 && usage.reasoning_tokens.is_none()
    {
        return NormalizedUsage {
            input_units: None,
            output_units: None,
            source: UsageSource::Unknown,
        };
    }
    NormalizedUsage {
        input_units: Some(u64::from(usage.prompt_tokens)),
        output_units: Some(u64::from(usage.completion_tokens)),
        source: UsageSource::Reported,
    }
}

fn parse_subprocess_frames(
    text: &str,
    request: &ConformanceRequest<'_>,
    diagnostics: Option<&DiagnosticSink>,
) -> Result<ConformanceOutcome, ConformanceError> {
    let mut content = String::new();
    let mut usage = None;
    let mut terminal = false;
    let mut item_count = 0_usize;

    for line in text.lines() {
        item_count = item_count.saturating_add(1);
        if item_count > MAX_REFERENCE_OUTPUT_ITEMS {
            return Err(ConformanceError::OutputExceeded);
        }
        if terminal {
            emit_provider_diagnostic(
                diagnostics,
                "subprocess-reference",
                DiagnosticReason::ProviderMalformedFrame,
                DiagnosticSeverity::Error,
            );
            return Err(ConformanceError::MalformedFrame);
        }
        let mut fields = line.split('\t');
        match fields.next() {
            Some("text") => {
                let Some(delta) = fields.next() else {
                    return malformed(diagnostics);
                };
                if fields.next().is_some() {
                    return malformed(diagnostics);
                }
                content.push_str(delta);
            }
            Some("tool") => {
                let Some(action) = fields.next() else {
                    return malformed(diagnostics);
                };
                if fields.next().is_some() || !request.action_grants.contains(action) {
                    return Err(ConformanceError::ActionGrantRequired {
                        action: action.to_owned(),
                    });
                }
            }
            Some("usage") => {
                let Some(input) = fields.next() else {
                    return malformed(diagnostics);
                };
                let Some(output) = fields.next() else {
                    return malformed(diagnostics);
                };
                let Some(source) = fields.next() else {
                    return malformed(diagnostics);
                };
                if fields.next().is_some() || usage.is_some() {
                    return malformed(diagnostics);
                }
                usage = Some(NormalizedUsage {
                    input_units: parse_optional_u64(input)?,
                    output_units: parse_optional_u64(output)?,
                    source: parse_usage_source(source)?,
                });
            }
            Some("done") => {
                if fields.next().is_some() {
                    return malformed(diagnostics);
                }
                terminal = true;
            }
            _ => return malformed(diagnostics),
        }
    }

    if !terminal {
        return malformed(diagnostics);
    }
    Ok(ConformanceOutcome {
        content,
        usage: usage.unwrap_or(NormalizedUsage {
            input_units: None,
            output_units: None,
            source: UsageSource::Unknown,
        }),
    })
}

fn parse_optional_u64(value: &str) -> Result<Option<u64>, ConformanceError> {
    if value == "-" {
        return Ok(None);
    }
    value
        .parse::<u64>()
        .map(Some)
        .map_err(|_| ConformanceError::MalformedFrame)
}

fn parse_usage_source(value: &str) -> Result<UsageSource, ConformanceError> {
    match value {
        "reported" => Ok(UsageSource::Reported),
        "derived" => Ok(UsageSource::Derived),
        "estimated" => Ok(UsageSource::Estimated),
        "unknown" => Ok(UsageSource::Unknown),
        _ => Err(ConformanceError::MalformedFrame),
    }
}

fn malformed(diagnostics: Option<&DiagnosticSink>) -> Result<ConformanceOutcome, ConformanceError> {
    emit_provider_diagnostic(
        diagnostics,
        "subprocess-reference",
        DiagnosticReason::ProviderMalformedFrame,
        DiagnosticSeverity::Error,
    );
    Err(ConformanceError::MalformedFrame)
}

fn map_provider_error(
    error: ProviderError,
    diagnostics: Option<&DiagnosticSink>,
) -> ConformanceError {
    match error {
        ProviderError::DeadlineExceeded => ConformanceError::DeadlineExceeded,
        ProviderError::Cancelled => {
            emit_provider_diagnostic(
                diagnostics,
                "http-reference",
                DiagnosticReason::ProviderCancelled,
                DiagnosticSeverity::Info,
            );
            ConformanceError::Cancelled
        }
        ProviderError::OutputExceeded => ConformanceError::OutputExceeded,
        ProviderError::ProtocolViolation { .. } => {
            emit_provider_diagnostic(
                diagnostics,
                "http-reference",
                DiagnosticReason::ProviderMalformedFrame,
                DiagnosticSeverity::Error,
            );
            ConformanceError::MalformedFrame
        }
        ProviderError::TransportUnavailable { .. }
        | ProviderError::UpstreamUnavailable { .. }
        | ProviderError::RateLimited { .. } => ConformanceError::TransportUnavailable,
        ProviderError::InvalidRequest { .. } => ConformanceError::ExecutionFailed,
    }
}

fn map_transport_build_error(error: TransportBuildError) -> ConformanceError {
    match error {
        TransportBuildError::InvalidCredential | TransportBuildError::ClientBuild => {
            ConformanceError::CredentialUnavailable
        }
    }
}

fn emit_provider_diagnostic(
    diagnostics: Option<&DiagnosticSink>,
    provider_id: &str,
    reason: DiagnosticReason,
    severity: DiagnosticSeverity,
) {
    let Some(sink) = diagnostics else {
        return;
    };
    let attributes = [SafeAttribute::new(
        SafeAttributeKey::ProviderId,
        provider_id,
    )];
    let _ = sink.emit(
        DiagnosticComponent::ProviderHost,
        provider_id,
        DiagnosticFamily::ProviderProtocol,
        reason,
        severity,
        &attributes,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_debug_never_contains_secret() {
        let credential = CredentialMaterial::new("credential://fixture", "super-secret-value");
        let rendered = format!("{credential:?}");
        assert!(rendered.contains("credential://fixture"));
        assert!(!rendered.contains("super-secret-value"));
    }

    #[test]
    fn subprocess_usage_preserves_provenance() {
        let grants = BTreeSet::new();
        let request = ConformanceRequest {
            model: "fixture",
            prompt: "hello",
            deadline: Duration::from_secs(1),
            cancellation: CancellationToken::new(),
            credential: None,
            required_action_grant: None,
            action_grants: &grants,
        };
        let parsed = parse_subprocess_frames(
            "text\thello\nusage\t3\t5\testimated\ndone\n",
            &request,
            None,
        );
        assert!(parsed.is_ok());
        let Ok(parsed) = parsed else {
            return;
        };
        assert_eq!(parsed.usage.source, UsageSource::Estimated);
        assert_eq!(parsed.usage.input_units, Some(3));
        assert_eq!(parsed.usage.output_units, Some(5));
    }

    #[test]
    fn malformed_frame_is_never_success() {
        let grants = BTreeSet::new();
        let request = ConformanceRequest {
            model: "fixture",
            prompt: "hello",
            deadline: Duration::from_secs(1),
            cancellation: CancellationToken::new(),
            credential: None,
            required_action_grant: None,
            action_grants: &grants,
        };
        assert_eq!(
            parse_subprocess_frames("text\thello\n", &request, None),
            Err(ConformanceError::MalformedFrame)
        );
    }

    #[test]
    fn action_grant_is_adapter_independent_preflight() {
        let grants = BTreeSet::new();
        let request = ConformanceRequest {
            model: "fixture",
            prompt: "hello",
            deadline: Duration::from_secs(1),
            cancellation: CancellationToken::new(),
            credential: None,
            required_action_grant: Some("tool.echo"),
            action_grants: &grants,
        };
        assert_eq!(
            validate_request(&request),
            Err(ConformanceError::ActionGrantRequired {
                action: "tool.echo".to_owned(),
            })
        );
    }
}
