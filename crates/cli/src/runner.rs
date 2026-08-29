//! Production CLI orchestration.
//!
//! The binary delegates here so every invocation follows one reusable path:
//! registry path resolution -> typed argv binding -> local content materialize ->
//! verified Instance discovery -> authenticated local Principal handshake ->
//! bounded remote preflight -> query or durable submission/recovery -> render.

use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use application_contract::{
    ExecutionContext, TargetMaterialization, cli_path_tokens, commands_in_group, metadata_for_key,
    parse_bound_input, project_for_execution, resolve_cli_path,
};
use control_client::{ClientError, LocalControlClient};
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::{CommandPayload, OperationResult, OutputFormat};
use serde_json::{Value, json};

use crate::{
    Confirmation, Discovery, MachineRenderer, StreamEvent, SubmissionFlowError,
    materialize_content, submit_or_recover,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const START_POLL_INTERVAL: Duration = Duration::from_millis(25);
const START_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CliOutput {
    fn success(stdout: String) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            exit_code: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedInvocation {
    global_tokens: Vec<String>,
    command_tokens: Vec<String>,
    root_help: bool,
    root_version: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalPaths {
    discovery_root: PathBuf,
    runtime_root: PathBuf,
    journal_root: PathBuf,
}

impl LocalPaths {
    fn discover() -> Self {
        let discovery = Discovery::new();
        let discovery_root = discovery.base_path().to_path_buf();
        let dxbot_root = discovery_root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("dxbot-state"));
        Self {
            runtime_root: dxbot_root.join("runtime"),
            journal_root: discovery_root.clone(),
            discovery_root,
        }
    }
}

pub fn execute(args: &[String]) -> CliOutput {
    let format_hint = requested_format(args);
    match execute_inner(args) {
        Ok(output) => output,
        Err(error) => render_error(error, format_hint),
    }
}

fn execute_inner(args: &[String]) -> Result<CliOutput, DxbotError> {
    if args.is_empty() {
        return Ok(CliOutput::success(Discovery::show_help()));
    }
    let normalized = normalize_invocation(args)?;
    if normalized.root_help && normalized.command_tokens.is_empty() {
        return Ok(CliOutput::success(Discovery::show_help()));
    }
    if normalized.root_version && normalized.command_tokens.is_empty() {
        return Ok(render_version(format_from_tokens(&normalized.global_tokens)?));
    }
    if normalized.command_tokens.is_empty() {
        return Err(usage_error("missing command"));
    }
    let group = &normalized.command_tokens[0];
    if !commands_in_group(group).is_empty()
        && (normalized.command_tokens.len() == 1
            || normalized.command_tokens.get(1).is_some_and(|token| is_help(token)))
    {
        return Ok(CliOutput::success(render_group_help(group)));
    }
    resolve_command(normalized)
}

fn resolve_command(normalized: NormalizedInvocation) -> Result<CliOutput, DxbotError> {
    let resolved = resolve_cli_path(&normalized.command_tokens)?;
    let metadata = metadata_for_key(resolved.command_key)
        .ok_or_else(|| internal_error("resolved command has no registry metadata"))?;
    let command_args = &normalized.command_tokens[resolved.consumed_path_tokens..];
    if command_args.iter().any(|token| is_help(token)) {
        return Ok(CliOutput::success(render_command_help(metadata.command_key)));
    }

    let mut canonical_args =
        Vec::with_capacity(1 + normalized.global_tokens.len() + command_args.len());
    canonical_args.push(metadata.command_key.to_owned());
    canonical_args.extend(normalized.global_tokens);
    canonical_args.extend_from_slice(command_args);
    let mut input = parse_bound_input(&canonical_args)?;
    let format = input.global_options.format;

    match metadata.command_key {
        "version" => Ok(render_version(format)),
        "runtime-start" => start_runtime(&input, format),
        "runtime-status" => runtime_status(&input, format),
        "runtime-doctor" => runtime_doctor(&input, format),
        _ if metadata.kind == "C" => submit_command(&mut input, metadata.command_key, format),
        _ if metadata.kind == "Q" => query_command(&mut input, metadata.command_key, format),
        _ => Err(owner_unavailable(metadata.command_key, metadata.kind)),
    }
}

fn start_runtime(
    input: &application_contract::CliInput,
    format: OutputFormat,
) -> Result<CliOutput, DxbotError> {
    #[cfg(not(unix))]
    {
        let _ = (input, format);
        Err(incompatible_error(
            "P0 local Runtime start requires a Unix-domain control endpoint",
        ))
    }
    #[cfg(unix)]
    {
        let paths = LocalPaths::discover();
        let discovery = Discovery::at(paths.discovery_root.clone());
        let profile = input.global_options.profile.as_deref();
        let explicit_instance = input.global_options.instance.as_deref();
        let timeout = parse_timeout(input.global_options.timeout.as_deref())?;

        if let Ok(selected) = discovery.select_endpoint(profile, explicit_instance) {
            if connect_selected(&selected, START_CONNECT_TIMEOUT)
                .and_then(|client| client.handshake().map_err(client_error))
                .is_ok()
            {
                return Ok(render_value(
                    json!({
                        "status": "already-running",
                        "instance_id": selected.descriptor.instance_id,
                        "host_generation": selected.descriptor.host_generation,
                        "endpoint": selected.descriptor.endpoint,
                    }),
                    format,
                    Some("Runtime already running"),
                ));
            }
        }
        if explicit_instance.is_some() {
            return Err(error_with(
                ErrorCode::NotFound,
                ErrorCategory::Input,
                "runtime start cannot create an explicitly requested unknown InstanceId",
            ));
        }

        let executable = std::env::current_exe()
            .map_err(|error| local_error(format!("cannot locate dxb executable: {error}")))?;
        let mut command = Command::new(executable);
        command
            .arg("__runtime-host")
            .arg("--runtime-root")
            .arg(&paths.runtime_root)
            .arg("--discovery-root")
            .arg(&paths.discovery_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(profile) = profile {
            command.arg("--profile").arg(profile);
        }
        let mut child = command
            .spawn()
            .map_err(|error| local_error(format!("cannot start Runtime Host process: {error}")))?;

        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(selected) = discovery.select_endpoint(profile, None) {
                if let Ok(client) = connect_selected(&selected, START_CONNECT_TIMEOUT) {
                    if client.handshake().is_ok() {
                        return Ok(render_value(
                            json!({
                                "status": "started",
                                "instance_id": selected.descriptor.instance_id,
                                "host_generation": selected.descriptor.host_generation,
                                "endpoint": selected.descriptor.endpoint,
                            }),
                            format,
                            Some("Runtime started"),
                        ));
                    }
                }
            }
            if let Some(status) = child
                .try_wait()
                .map_err(|error| local_error(format!("cannot inspect Runtime Host process: {error}")))?
            {
                return Err(runtime_unavailable(format!(
                    "Runtime Host exited before control readiness: {status}"
                )));
            }
            if Instant::now() >= deadline {
                return Err(timeout_error("Runtime Host did not reach control readiness"));
            }
            thread::sleep(START_POLL_INTERVAL);
        }
    }
}

fn runtime_status(
    input: &application_contract::CliInput,
    format: OutputFormat,
) -> Result<CliOutput, DxbotError> {
    #[cfg(not(unix))]
    {
        let _ = (input, format);
        Err(incompatible_error(
            "P0 local Runtime status requires a Unix-domain control endpoint",
        ))
    }
    #[cfg(unix)]
    {
        let (selected, client, handshake) = authenticated_client(input)?;
        Ok(render_value(
            json!({
                "status": "running",
                "instance_id": handshake.instance_id,
                "host_generation": handshake.host_generation,
                "endpoint": selected.descriptor.endpoint,
                "protocol_version": handshake.protocol_version,
                "schema_version": handshake.schema_version,
                "provider_id": selected.descriptor.provider_id,
                "provider_ready": selected.descriptor.provider_ready,
            }),
            format,
            Some("Runtime running"),
        ))
    }
}

fn runtime_doctor(
    input: &application_contract::CliInput,
    format: OutputFormat,
) -> Result<CliOutput, DxbotError> {
    #[cfg(not(unix))]
    {
        let _ = (input, format);
        Err(incompatible_error(
            "P0 Runtime doctor requires a Unix-domain control endpoint",
        ))
    }
    #[cfg(unix)]
    {
        let paths = LocalPaths::discover();
        let discovery = Discovery::at(paths.discovery_root);
        let (selected, _client, handshake) = authenticated_client(input)?;
        let provider = discovery
            .doctor_provider(&handshake.instance_id)
            .map_err(|error| error.to_dxbot_error())?;
        Ok(render_value(
            json!({
                "instance_id": handshake.instance_id,
                "host_generation": handshake.host_generation,
                "endpoint": selected.descriptor.endpoint,
                "control": "ready",
                "provider": {
                    "provider_id": provider.provider_id,
                    "status": provider.status,
                    "required_capabilities": provider.required_capabilities,
                    "available": provider.available,
                }
            }),
            format,
            Some("Runtime doctor completed"),
        ))
    }
}

fn query_command(
    input: &mut application_contract::CliInput,
    command_key: &str,
    format: OutputFormat,
) -> Result<CliOutput, DxbotError> {
    #[cfg(not(unix))]
    {
        let _ = (input, command_key, format);
        Err(incompatible_error(
            "P0 query requires a Unix-domain control endpoint",
        ))
    }
    #[cfg(unix)]
    {
        let (_selected, client, handshake) = authenticated_client(input)?;
        let payload = prepare_remote_payload(input, &client, &handshake)?;
        let value = client.query(&payload).map_err(client_error)?;
        Ok(render_value(value, format, Some(command_key)))
    }
}

fn submit_command(
    input: &mut application_contract::CliInput,
    command_key: &str,
    format: OutputFormat,
) -> Result<CliOutput, DxbotError> {
    #[cfg(not(unix))]
    {
        let _ = (input, command_key, format);
        Err(incompatible_error(
            "P0 command submission requires a Unix-domain control endpoint",
        ))
    }
    #[cfg(unix)]
    {
        materialize_content(input)?;
        let paths = LocalPaths::discover();
        let (_selected, client, handshake) = authenticated_client(input)?;

        if metadata_for_key(command_key)
            .is_some_and(|metadata| metadata.typed_fields.contains("confirmation:"))
        {
            let target = input
                .selector_value()
                .unwrap_or_else(|| handshake.instance_id.0.clone());
            let confirmed = Confirmation::require_destructive(
                command_key,
                &target,
                std::io::stdin().is_terminal(),
                input.global_options.yes,
            )
            .map_err(|error| error.to_dxbot_error())?;
            if !confirmed {
                return Err(interrupted_error("operation cancelled by user"));
            }
        }

        let payload = prepare_remote_payload(input, &client, &handshake)?;
        let result = submit_or_recover(input, payload, &paths.journal_root, client)
            .map_err(submission_flow_error)?;
        Ok(render_operation(&result, command_key, format))
    }
}

#[cfg(unix)]
fn authenticated_client(
    input: &application_contract::CliInput,
) -> Result<(
    crate::discovery::SelectedEndpoint,
    LocalControlClient,
    application_contract::LocalControlHandshake,
), DxbotError> {
    let paths = LocalPaths::discover();
    let discovery = Discovery::at(paths.discovery_root);
    let selected = discovery
        .select_endpoint(
            input.global_options.profile.as_deref(),
            input.global_options.instance.as_deref(),
        )
        .map_err(|error| error.to_dxbot_error())?;
    let timeout = parse_timeout(input.global_options.timeout.as_deref())?;
    let client = connect_selected(&selected, timeout)?;
    let handshake = client.handshake().map_err(client_error)?;
    Ok((selected, client, handshake))
}

#[cfg(unix)]
fn prepare_remote_payload(
    input: &mut application_contract::CliInput,
    client: &LocalControlClient,
    handshake: &application_contract::LocalControlHandshake,
) -> Result<CommandPayload, DxbotError> {
    let context = ExecutionContext::new(
        handshake.instance_id.clone(),
        handshake.principal_ref.clone(),
    );
    let mut payload = project_for_execution(input, &context)?;
    let target = TargetMaterialization::from_cli_input(input)?;
    let plan = target.plan_preflight(input);
    if input.selector.is_some() || !plan.queries.is_empty() {
        let (canonical_target, cas) = client
            .preflight(&payload, input.selector.clone())
            .map_err(client_error)?;
        input.cas = Some(cas);
        payload.canonical_target = canonical_target;
        payload.cas = Some(cas);
    }
    Ok(payload)
}

#[cfg(unix)]
fn connect_selected(
    selected: &crate::discovery::SelectedEndpoint,
    timeout: Duration,
) -> Result<LocalControlClient, DxbotError> {
    LocalControlClient::new(
        selected
            .unix_socket_path()
            .map_err(|error| error.to_dxbot_error())?,
        selected.descriptor.instance_id.clone(),
        selected.descriptor.host_generation,
        selected.state_owner_uid,
        timeout,
    )
    .map_err(client_error)
}

fn render_operation(result: &OperationResult, command_key: &str, format: OutputFormat) -> CliOutput {
    let renderer = MachineRenderer::new();
    match format {
        OutputFormat::Human => CliOutput::success(format!(
            "{command_key}: {} (operation {})\n",
            result.status, result.operation_id.0
        )),
        OutputFormat::Json => {
            let mut output = renderer.render_json(result);
            output.push('\n');
            CliOutput::success(output)
        }
        OutputFormat::Jsonl => CliOutput::success(renderer.render_jsonl(&[
            StreamEvent::Terminal(Box::new(result.clone())),
        ])),
    }
}

fn render_version(format: OutputFormat) -> CliOutput {
    let version = Discovery::show_version();
    match format {
        OutputFormat::Human => CliOutput::success(format!(
            "dxb {} (protocol {}, schema {})\n",
            version.client_version,
            version
                .supported_protocol_versions
                .first()
                .map(String::as_str)
                .unwrap_or("unknown"),
            version
                .supported_schema_versions
                .first()
                .map(String::as_str)
                .unwrap_or("unknown")
        )),
        OutputFormat::Json | OutputFormat::Jsonl => {
            let mut serialized = serde_json::to_string(&version).unwrap_or_else(|error| {
                json!({"error": {"code": "internal-invariant", "message": error.to_string()}})
                    .to_string()
            });
            serialized.push('\n');
            CliOutput::success(serialized)
        }
    }
}

fn render_value(value: Value, format: OutputFormat, human: Option<&str>) -> CliOutput {
    match format {
        OutputFormat::Human => {
            let headline = human.unwrap_or("ok");
            CliOutput::success(format!(
                "{headline}\n{}\n",
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
            ))
        }
        OutputFormat::Json | OutputFormat::Jsonl => {
            let mut output = value.to_string();
            output.push('\n');
            CliOutput::success(output)
        }
    }
}

fn render_error(error: DxbotError, format: OutputFormat) -> CliOutput {
    let renderer = MachineRenderer::new();
    let mut rendered = renderer.render_error(&error, format);
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    match format {
        OutputFormat::Human => CliOutput {
            stdout: String::new(),
            stderr: rendered,
            exit_code: error.code.exit_code(),
        },
        OutputFormat::Json | OutputFormat::Jsonl => CliOutput {
            stdout: rendered,
            stderr: String::new(),
            exit_code: error.code.exit_code(),
        },
    }
}

fn normalize_invocation(args: &[String]) -> Result<NormalizedInvocation, DxbotError> {
    let mut global_tokens = Vec::new();
    let mut command_tokens = Vec::new();
    let mut seen = HashSet::new();
    let mut root_help = false;
    let mut root_version = false;
    let mut after_separator = false;
    let mut index = 0usize;

    while index < args.len() {
        let token = &args[index];
        if after_separator {
            command_tokens.push(token.clone());
            index += 1;
            continue;
        }
        if token == "--" {
            after_separator = true;
            command_tokens.push(token.clone());
            index += 1;
            continue;
        }
        match token.as_str() {
            "--profile" | "--instance" | "--format" | "--color" | "--wait" | "--timeout" => {
                if !seen.insert(token.clone()) {
                    return Err(usage_error(format!("duplicate global option: {token}")));
                }
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| usage_error(format!("{token} requires a value")))?;
                global_tokens.push(token.clone());
                global_tokens.push(value.clone());
                index += 2;
            }
            "-y" | "--yes" => {
                if !seen.insert("--yes".to_owned()) {
                    return Err(usage_error("duplicate global option: --yes"));
                }
                global_tokens.push("--yes".to_owned());
                index += 1;
            }
            "-h" | "--help" if command_tokens.is_empty() => {
                root_help = true;
                index += 1;
            }
            "--version" if command_tokens.is_empty() => {
                root_version = true;
                index += 1;
            }
            _ => {
                command_tokens.push(token.clone());
                index += 1;
            }
        }
    }
    Ok(NormalizedInvocation {
        global_tokens,
        command_tokens,
        root_help,
        root_version,
    })
}

fn requested_format(args: &[String]) -> OutputFormat {
    args.windows(2)
        .find_map(|window| {
            if window[0] == "--format" {
                match window[1].as_str() {
                    "json" => Some(OutputFormat::Json),
                    "jsonl" => Some(OutputFormat::Jsonl),
                    "human" => Some(OutputFormat::Human),
                    _ => None,
                }
            } else {
                None
            }
        })
        .unwrap_or(OutputFormat::Human)
}

fn format_from_tokens(tokens: &[String]) -> Result<OutputFormat, DxbotError> {
    let args = std::iter::once("version".to_owned())
        .chain(tokens.iter().cloned())
        .collect::<Vec<_>>();
    parse_bound_input(&args).map(|input| input.global_options.format)
}

fn parse_timeout(value: Option<&str>) -> Result<Duration, DxbotError> {
    let Some(value) = value else {
        return Ok(DEFAULT_TIMEOUT);
    };
    let (number, multiplier_ms) = if let Some(value) = value.strip_suffix("ms") {
        (value, 1_u64)
    } else if let Some(value) = value.strip_suffix('s') {
        (value, 1_000)
    } else if let Some(value) = value.strip_suffix('m') {
        (value, 60_000)
    } else if let Some(value) = value.strip_suffix('h') {
        (value, 3_600_000)
    } else {
        (value, 1_000)
    };
    let number = number
        .parse::<u64>()
        .map_err(|_| input_error(format!("invalid timeout: {value}")))?;
    let millis = number
        .checked_mul(multiplier_ms)
        .ok_or_else(|| input_error("timeout is too large"))?;
    let duration = Duration::from_millis(millis);
    if duration.is_zero() || duration > MAX_TIMEOUT {
        return Err(input_error(format!(
            "timeout must be greater than zero and at most {} seconds",
            MAX_TIMEOUT.as_secs()
        )));
    }
    Ok(duration)
}

fn render_group_help(group: &str) -> String {
    let commands = commands_in_group(group);
    let mut output = format!("dxb {group} commands:\n");
    for metadata in commands {
        let path = cli_path_tokens(metadata.command_key).join(" ");
        output.push_str(&format!(
            "  dxb {path:<32} kind={} wait={}\n",
            metadata.kind, metadata.wait_default
        ));
    }
    output
}

fn render_command_help(command_key: &str) -> String {
    let Some(metadata) = metadata_for_key(command_key) else {
        return format!("unknown command: {command_key}\n");
    };
    let path = cli_path_tokens(command_key).join(" ");
    format!(
        "dxb {path}\n\ncommand-key: {}\nkind: {}\ninput: {}\ntarget: {}\noutput: {}\nsecurity: {}\nwait: default={}, allowed={}\nfields: {}\n",
        metadata.command_key,
        metadata.kind,
        metadata.input_schema,
        metadata.target_schema,
        metadata.output_schema,
        metadata.security_class,
        metadata.wait_default,
        metadata.wait_allowed,
        metadata.typed_fields
    )
}

fn is_help(token: &str) -> bool {
    matches!(token, "-h" | "--help")
}

fn submission_flow_error(error: SubmissionFlowError) -> DxbotError {
    match error {
        SubmissionFlowError::Contract(error) => error,
        SubmissionFlowError::Journal(error) => {
            storage_error(format!("local journal recovery failed: {error:?}"))
        }
        SubmissionFlowError::Client(error) => client_error(error),
        SubmissionFlowError::AmbiguousRecovery(commands) => error_with(
            ErrorCode::RecoveryRequired,
            ErrorCategory::Recovery,
            format!("multiple non-terminal commands match this invocation: {commands:?}"),
        ),
        SubmissionFlowError::PreparedAlreadyBound(command_id) => error_with(
            ErrorCode::InternalInvariant,
            ErrorCategory::Internal,
            format!(
                "Prepared command unexpectedly has a server binding: {}",
                command_id.0
            ),
        ),
        SubmissionFlowError::ObservedBindingMissing(command_id) => error_with(
            ErrorCode::RecoveryRequired,
            ErrorCategory::Recovery,
            format!(
                "Observed command has no canonical server binding: {}",
                command_id.0
            ),
        ),
    }
}

fn client_error(error: ClientError) -> DxbotError {
    match error {
        ClientError::Remote(error) => error,
        ClientError::TransportUnavailable | ClientError::Transport(_) => {
            runtime_unavailable(error.to_string())
        }
        ClientError::Storage(message) => storage_error(message),
        ClientError::RecoveryRequired(command_id) => error_with(
            ErrorCode::RecoveryRequired,
            ErrorCategory::Recovery,
            format!("recovery required for command {}", command_id.0),
        ),
        ClientError::AlreadyExists(command_id)
        | ClientError::IdempotencyKeyConflict(command_id)
        | ClientError::RequestDigestConflict(command_id)
        | ClientError::OperationIdConflict(command_id)
        | ClientError::TransportIdentityConflict(command_id) => error_with(
            ErrorCode::Conflict,
            ErrorCategory::Conflict,
            format!("operation identity conflict for command {}", command_id.0),
        ),
        ClientError::NotCommitted(command_id) => error_with(
            ErrorCode::RecoveryRequired,
            ErrorCategory::Recovery,
            format!("command {} is not terminal", command_id.0),
        ),
        ClientError::InstanceMismatch { client, request } => incompatible_error(format!(
            "Runtime Instance mismatch: client={}, request={}",
            client.0, request.0
        )),
        ClientError::SimulatedCrash(point) => {
            internal_error(format!("simulated crash escaped production path: {point:?}"))
        }
    }
}

fn owner_unavailable(command_key: &str, kind: &str) -> DxbotError {
    error_with(
        ErrorCode::InternalInvariant,
        ErrorCategory::Internal,
        format!("registry command {command_key} ({kind}) has no connected production owner"),
    )
}
fn usage_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::Usage, ErrorCategory::Input, message)
}
fn input_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::InvalidInput, ErrorCategory::Input, message)
}
fn local_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::StorageOrCorruption, ErrorCategory::Local, message)
}
fn storage_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::StorageOrCorruption, ErrorCategory::Integrity, message)
}
fn runtime_unavailable(message: impl Into<String>) -> DxbotError {
    error_with(
        ErrorCode::RuntimeUnavailable,
        ErrorCategory::Availability,
        message,
    )
}
fn incompatible_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::Incompatible, ErrorCategory::Conflict, message)
}
fn timeout_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::Timeout, ErrorCategory::Availability, message)
}
fn interrupted_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::Interrupted, ErrorCategory::Local, message)
}
fn internal_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::InternalInvariant, ErrorCategory::Internal, message)
}
fn error_with(
    code: ErrorCode,
    category: ErrorCategory,
    message: impl Into<String>,
) -> DxbotError {
    DxbotError {
        code,
        category,
        message: message.into(),
        retryable: false,
        operation_ref: None,
        target_refs: Vec::new(),
        field_violations: Vec::new(),
        current_revision: None,
        current_generation: None,
        resume_cursor: None,
        next_actions: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn root_help_is_offline_and_successful() {
        let output = execute(&args(&["--help"]));
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("dxb — DXBOT"));
    }

    #[test]
    fn group_help_is_registry_driven_and_offline() {
        let output = execute(&args(&["task", "--help"]));
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("dxb task submit"));
        assert!(output.stdout.contains("dxb task redirect"));
    }

    #[test]
    fn global_options_are_order_independent_around_command_path() {
        let output = execute(&args(&["bot", "--format", "json", "--help"]));
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("dxb bot commands"));
    }

    #[test]
    fn duplicate_global_option_is_rejected() {
        let output = execute(&args(&[
            "--format", "json", "bot", "--format", "human", "list",
        ]));
        assert_eq!(output.exit_code, ErrorCode::Usage.exit_code());
    }

    #[test]
    fn version_json_uses_same_protocol_constants_as_transport() {
        let output = execute(&args(&["--format", "json", "--version"]));
        assert_eq!(output.exit_code, 0);
        let value: Value = serde_json::from_str(output.stdout.trim()).expect("valid json");
        assert_eq!(value["supported_protocol_versions"][0], "1");
        assert_eq!(value["supported_schema_versions"][0], "v1");
    }

    #[test]
    fn timeout_parser_is_bounded() {
        assert_eq!(
            parse_timeout(Some("250ms")).expect("timeout parses"),
            Duration::from_millis(250)
        );
        assert!(parse_timeout(Some("11m")).is_err());
        assert!(parse_timeout(Some("0s")).is_err());
    }
}
