//! Production CLI orchestration.
//!
//! Every invocation follows one reusable path:
//! registry path resolution -> typed argv binding -> local content materialize ->
//! verified Instance discovery -> authenticated local Principal handshake ->
//! bounded preflight -> query, stream, host action or durable submission/recovery
//! -> render. Local-only options never cross the Prepared boundary.

use std::collections::HashSet;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use application_contract::{
    CliInput, ExecutionContext, TargetMaterialization, cli_path_tokens, commands_in_group,
    metadata_for_key, parse_bound_input, project_for_execution, resolve_cli_path,
};
use control_client::{ClientError, LocalControlClient};
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::{ColorMode, CommandPayload, OperationResult, OutputFormat};
use serde_json::{Value, json};

use crate::{
    Confirmation, Discovery, MachineRenderer, SafeWriter, StreamEvent, SubmissionFlowError,
    materialize_content, submit_or_recover,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const START_POLL_INTERVAL: Duration = Duration::from_millis(25);
const START_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
const WATCH_POLL_MAX: Duration = Duration::from_secs(1);
const MAX_ALL_PAGES: usize = 10_000;
const MAX_ALL_ITEMS: usize = 100_000;
const MAX_ACCUMULATED_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_DIAGNOSTIC_PAGE_SIZE: usize = 50;
const MAX_DIAGNOSTIC_PAGE_SIZE: usize = 1000;
const DIAGNOSTIC_CURSOR_PREFIX: &str = "d";

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

#[derive(Debug, Clone, PartialEq)]
enum PreparedInvocation {
    Offline(CliOutput),
    Command {
        input: CliInput,
        command_key: String,
        kind: String,
    },
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

/// Deterministic buffered execution surface used by tests and embedders.
/// Streaming commands require [`run_process`] because buffering an unbounded
/// stream would violate the CLI contract.
pub fn execute(args: &[String]) -> CliOutput {
    let format_hint = requested_format(args);
    let color_enabled =
        format_hint == OutputFormat::Human && requested_color(args) == ColorMode::Always;
    match prepare_invocation(args).and_then(execute_prepared_buffered) {
        Ok(output) => decorate_output(output, color_enabled),
        Err(error) => decorate_output(render_error(error, format_hint), color_enabled),
    }
}

/// Production process entrypoint. Bounded commands reuse the same buffered
/// path; S-kind commands write and flush each observation directly to stdout.
pub fn run_process(args: &[String]) -> i32 {
    let format_hint = requested_format(args);
    let stdout_is_terminal = std::io::stdout().is_terminal();
    let color_enabled = format_hint == OutputFormat::Human
        && match requested_color(args) {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => stdout_is_terminal,
        };

    match prepare_invocation(args) {
        Ok(PreparedInvocation::Command {
            mut input,
            command_key,
            kind,
        }) if kind == "S" => match stream_command(&mut input, &command_key, color_enabled) {
            Ok(()) => 0,
            Err(error) => emit_cli_output(&decorate_output(
                render_error(error, format_hint),
                color_enabled,
            )),
        },
        Ok(prepared) => match execute_prepared_buffered(prepared) {
            Ok(output) => emit_cli_output(&decorate_output(output, color_enabled)),
            Err(error) => emit_cli_output(&decorate_output(
                render_error(error, format_hint),
                color_enabled,
            )),
        },
        Err(error) => emit_cli_output(&decorate_output(
            render_error(error, format_hint),
            color_enabled,
        )),
    }
}

fn prepare_invocation(args: &[String]) -> Result<PreparedInvocation, DxbotError> {
    if args.is_empty() {
        return Ok(PreparedInvocation::Offline(CliOutput::success(
            Discovery::show_help(),
        )));
    }
    let normalized = normalize_invocation(args)?;
    if normalized.root_help && normalized.command_tokens.is_empty() {
        return Ok(PreparedInvocation::Offline(CliOutput::success(
            Discovery::show_help(),
        )));
    }
    if normalized.root_version && normalized.command_tokens.is_empty() {
        return Ok(PreparedInvocation::Offline(render_version(
            format_from_tokens(&normalized.global_tokens)?,
        )));
    }
    if normalized.command_tokens.is_empty() {
        return Err(usage_error("missing command"));
    }

    let group = &normalized.command_tokens[0];
    if !commands_in_group(group).is_empty()
        && (normalized.command_tokens.len() == 1
            || normalized
                .command_tokens
                .get(1)
                .is_some_and(|token| is_help(token)))
    {
        return Ok(PreparedInvocation::Offline(CliOutput::success(
            render_group_help(group),
        )));
    }

    let resolved = resolve_cli_path(&normalized.command_tokens)?;
    let metadata = metadata_for_key(resolved.command_key)
        .ok_or_else(|| internal_error("resolved command has no registry metadata"))?;
    let command_args = &normalized.command_tokens[resolved.consumed_path_tokens..];
    if command_args.iter().any(|token| is_help(token)) {
        return Ok(PreparedInvocation::Offline(CliOutput::success(
            render_command_help(metadata.command_key),
        )));
    }

    let mut canonical_args =
        Vec::with_capacity(1 + normalized.global_tokens.len() + command_args.len());
    canonical_args.push(metadata.command_key.to_owned());
    canonical_args.extend(normalized.global_tokens);
    canonical_args.extend_from_slice(command_args);
    let input = parse_bound_input(&canonical_args)?;
    Ok(PreparedInvocation::Command {
        input,
        command_key: metadata.command_key.to_owned(),
        kind: metadata.kind.to_owned(),
    })
}

fn execute_prepared_buffered(prepared: PreparedInvocation) -> Result<CliOutput, DxbotError> {
    match prepared {
        PreparedInvocation::Offline(output) => Ok(output),
        PreparedInvocation::Command {
            mut input,
            command_key,
            kind,
        } => {
            let format = input.global_options.format;
            match command_key.as_str() {
                "version" => Ok(render_version(format)),
                "runtime-start" => start_runtime(&input, format),
                "runtime-status" => runtime_status(&input, format),
                "runtime-stop-host" => stop_runtime_host(&input, format),
                "runtime-doctor" => runtime_doctor(&input, format),
                _ if kind == "C" => submit_command(&mut input, &command_key, format),
                _ if kind == "Q" => query_command(&mut input, &command_key, format),
                _ if kind == "S" => Err(input_error(
                    "streaming commands require the production process entrypoint",
                )),
                _ => Err(owner_unavailable(&command_key, &kind)),
            }
        }
    }
}

fn start_runtime(input: &CliInput, format: OutputFormat) -> Result<CliOutput, DxbotError> {
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
            if let Some(status) = child.try_wait().map_err(|error| {
                local_error(format!("cannot inspect Runtime Host process: {error}"))
            })? {
                return Err(runtime_unavailable(format!(
                    "Runtime Host exited before control readiness: {status}"
                )));
            }
            if Instant::now() >= deadline {
                return Err(timeout_error(
                    "Runtime Host did not reach control readiness",
                ));
            }
            thread::sleep(START_POLL_INTERVAL);
        }
    }
}

fn runtime_status(input: &CliInput, format: OutputFormat) -> Result<CliOutput, DxbotError> {
    #[cfg(not(unix))]
    {
        let _ = (input, format);
        Err(incompatible_error(
            "P0 local Runtime status requires a Unix-domain control endpoint",
        ))
    }
    #[cfg(unix)]
    {
        let (selected, _client, handshake) = authenticated_client(input)?;
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

fn stop_runtime_host(input: &CliInput, format: OutputFormat) -> Result<CliOutput, DxbotError> {
    #[cfg(not(unix))]
    {
        let _ = (input, format);
        Err(incompatible_error(
            "P0 Runtime host stop requires a Unix-domain control endpoint",
        ))
    }
    #[cfg(unix)]
    {
        let (_selected, client, handshake) = authenticated_client(input)?;
        let confirmed = Confirmation::require_destructive(
            "runtime-stop-host",
            &handshake.instance_id.0,
            std::io::stdin().is_terminal(),
            input.global_options.yes,
        )
        .map_err(|error| error.to_dxbot_error())?;
        if !confirmed {
            return Err(interrupted_error("host stop cancelled by user"));
        }
        let requested_generation = input
            .cas
            .and_then(|cas| cas.if_host_generation)
            .unwrap_or(handshake.host_generation);
        let value = client
            .stop_host(requested_generation)
            .map_err(client_error)?;
        Ok(render_value(value, format, Some("Runtime host stopped")))
    }
}

fn runtime_doctor(input: &CliInput, format: OutputFormat) -> Result<CliOutput, DxbotError> {
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
        let (_selected, _client, handshake) = authenticated_client(input)?;
        let provider = discovery
            .doctor_provider(&handshake.instance_id)
            .map_err(|error| error.to_dxbot_error())?;
        let diagnostics = vec![
            json!({
                "section": "control",
                "status": "ready",
                "instance_id": handshake.instance_id,
                "host_generation": handshake.host_generation,
                "endpoint_verified": true,
            }),
            json!({
                "section": "journal",
                "status": if paths.journal_root.is_dir() { "ready" } else { "unavailable" },
                "available": paths.journal_root.is_dir(),
            }),
            json!({
                "section": "provider",
                "provider_id": provider.provider_id,
                "status": provider.status,
                "required_capabilities": provider.required_capabilities,
                "available": provider.available,
            }),
        ];
        let value = diagnostic_page(input, diagnostics)?;
        Ok(render_value(
            value,
            format,
            Some("Runtime doctor completed"),
        ))
    }
}

fn query_command(
    input: &mut CliInput,
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
        let mut payload = prepare_remote_payload(input, &client, &handshake)?;
        let value = if local_bool(input, "all")? {
            if payload.semantic_options.get("cursor").is_some() {
                return Err(input_error("--all cannot be combined with --cursor"));
            }
            query_all_pages(&client, &mut payload)?
        } else {
            client.query(&payload).map_err(client_error)?
        };

        if let Some(path) = local_string(input, "output")? {
            return write_output_artifact(&path, &value, format, command_key);
        }
        Ok(render_value(value, format, Some(command_key)))
    }
}

#[cfg(unix)]
fn query_all_pages(
    client: &LocalControlClient,
    payload: &mut CommandPayload,
) -> Result<Value, DxbotError> {
    let mut items = Vec::new();
    let mut pages = 0usize;
    let mut bytes = 0usize;
    let mut cursor: Option<String> = None;

    loop {
        pages = pages
            .checked_add(1)
            .ok_or_else(|| internal_error("query page counter exhausted"))?;
        if pages > MAX_ALL_PAGES {
            return Err(partial_error(
                "--all exceeded the maximum page count",
                cursor,
            ));
        }
        set_payload_cursor(payload, cursor.as_deref())?;
        let page = client.query(payload).map_err(client_error)?;
        bytes = bytes
            .checked_add(page.to_string().len())
            .ok_or_else(|| internal_error("query byte counter exhausted"))?;
        if bytes > MAX_ACCUMULATED_BYTES {
            return Err(partial_error(
                "--all exceeded the maximum accumulated response size",
                page.get("next_cursor")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or(cursor),
            ));
        }
        let object = page
            .as_object()
            .ok_or_else(|| internal_error("paged query did not return an object"))?;
        let page_items = object
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(|| internal_error("paged query omitted items array"))?;
        if items.len().saturating_add(page_items.len()) > MAX_ALL_ITEMS {
            return Err(partial_error(
                "--all exceeded the maximum item count",
                object
                    .get("next_cursor")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or(cursor),
            ));
        }
        items.extend(page_items.iter().cloned());

        let has_more = object
            .get("has_more")
            .and_then(Value::as_bool)
            .ok_or_else(|| internal_error("paged query omitted has_more"))?;
        if !has_more {
            break;
        }
        let next = object
            .get("next_cursor")
            .and_then(Value::as_str)
            .ok_or_else(|| internal_error("paged query has_more without next_cursor"))?
            .to_owned();
        if cursor.as_deref() == Some(next.as_str()) {
            return Err(internal_error("paged query cursor made no progress"));
        }
        cursor = Some(next);
    }

    Ok(json!({
        "items": items,
        "next_cursor": Value::Null,
        "has_more": false,
        "pages_read": pages,
    }))
}

#[cfg(unix)]
fn set_payload_cursor(
    payload: &mut CommandPayload,
    cursor: Option<&str>,
) -> Result<(), DxbotError> {
    let object = payload
        .semantic_options
        .as_object_mut()
        .ok_or_else(|| internal_error("projected semantic_options must be a JSON object"))?;
    match cursor {
        Some(cursor) => {
            object.insert("cursor".to_owned(), Value::String(cursor.to_owned()));
        }
        None => {
            object.remove("cursor");
        }
    }
    Ok(())
}

fn write_output_artifact(
    path: &str,
    value: &Value,
    format: OutputFormat,
    command_key: &str,
) -> Result<CliOutput, DxbotError> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| internal_error(format!("cannot serialize query output: {error}")))?;
    bytes.push(b'\n');
    if bytes.len() > MAX_ACCUMULATED_BYTES {
        return Err(error_with(
            ErrorCode::ResourceExhausted,
            ErrorCategory::Resource,
            format!(
                "output is {} bytes, exceeding the {} byte local ceiling",
                bytes.len(),
                MAX_ACCUMULATED_BYTES
            ),
        ));
    }
    SafeWriter::new()
        .write_to_path(Path::new(path), &bytes, true)
        .map_err(|error| error.to_dxbot_error())?;
    Ok(render_value(
        json!({
            "command": command_key,
            "output": path,
            "bytes_written": bytes.len(),
        }),
        format,
        Some("Output written"),
    ))
}

fn submit_command(
    input: &mut CliInput,
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
        ensure_wait_satisfied(input, &result)?;
        Ok(render_operation(&result, command_key, format))
    }
}

fn ensure_wait_satisfied(input: &CliInput, result: &OperationResult) -> Result<(), DxbotError> {
    let wait = input.global_options.wait.as_deref().unwrap_or("committed");
    match wait {
        "none" | "accepted" | "committed" => Ok(()),
        "applied" if result.committed_payload.is_some() && !result.operation_may_continue => Ok(()),
        "applied" => Err(error_with(
            ErrorCode::PartialOrResync,
            ErrorCategory::Recovery,
            "operation committed but the requested applied wait point was not observed",
        )),
        other => Err(input_error(format!("unsupported wait point: {other}"))),
    }
}

fn stream_command(
    input: &mut CliInput,
    command_key: &str,
    color_enabled: bool,
) -> Result<(), DxbotError> {
    #[cfg(not(unix))]
    {
        let _ = (input, command_key, color_enabled);
        Err(incompatible_error(
            "P0 stream observation requires a Unix-domain control endpoint",
        ))
    }
    #[cfg(unix)]
    {
        if input.global_options.format == OutputFormat::Json {
            return Err(input_error(
                "streaming commands cannot use --format json; use human or jsonl",
            ));
        }
        let (_selected, client, handshake) = authenticated_client(input)?;
        let payload = prepare_remote_payload(input, &client, &handshake)?;
        let mut cursor = local_string(input, "cursor")?;
        let transport_timeout = parse_timeout(input.global_options.timeout.as_deref())?;
        let watch_timeout = watch_poll_timeout(transport_timeout);
        let deadline = input
            .global_options
            .timeout
            .as_ref()
            .map(|_| Instant::now() + transport_timeout);
        let mut stdout = std::io::stdout().lock();

        loop {
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                return Err(timeout_error(
                    "watch observation timed out locally; the runtime target was not cancelled",
                ));
            }
            let value = match client.watch_next(&payload, cursor.clone(), watch_timeout) {
                Ok(value) => value,
                Err(ClientError::Remote(error)) if error.code == ErrorCode::Timeout => continue,
                Err(error) => return Err(client_error(error)),
            };
            let next_cursor = value
                .get("cursor")
                .and_then(Value::as_str)
                .ok_or_else(|| internal_error("watch observation omitted cursor"))?
                .to_owned();
            if cursor
                .as_deref()
                .is_some_and(|current| current > next_cursor.as_str())
            {
                return Err(internal_error("watch cursor regressed"));
            }
            let terminal = value
                .get("terminal")
                .and_then(Value::as_bool)
                .ok_or_else(|| internal_error("watch observation omitted terminal flag"))?;
            let line = render_stream_value(
                &value,
                input.global_options.format,
                command_key,
                color_enabled,
            )?;
            stdout
                .write_all(line.as_bytes())
                .and_then(|()| stdout.flush())
                .map_err(|error| {
                    interrupted_error(format!(
                        "stream output closed; runtime observation stopped without cancelling target: {error}"
                    ))
                })?;
            cursor = Some(next_cursor);
            if terminal {
                return Ok(());
            }
        }
    }
}

fn watch_poll_timeout(transport_timeout: Duration) -> Duration {
    let half_ms = (transport_timeout.as_millis() / 2).max(1);
    let bounded_ms = half_ms.min(WATCH_POLL_MAX.as_millis());
    Duration::from_millis(u64::try_from(bounded_ms).unwrap_or(1))
}

fn render_stream_value(
    value: &Value,
    format: OutputFormat,
    command_key: &str,
    color_enabled: bool,
) -> Result<String, DxbotError> {
    match format {
        OutputFormat::Json => Err(input_error(
            "streaming commands cannot render a bounded JSON document",
        )),
        OutputFormat::Jsonl => Ok(format!("{value}\n")),
        OutputFormat::Human => {
            let event = value
                .get("event")
                .and_then(Value::as_str)
                .unwrap_or(command_key);
            let state = value
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let cursor = value.get("cursor").and_then(Value::as_str).unwrap_or("?");
            let headline = format!("{event}: state={state} cursor={cursor}");
            let headline = if color_enabled {
                format!("\x1b[36m{headline}\x1b[0m")
            } else {
                headline
            };
            Ok(format!("{headline}\n"))
        }
    }
}

#[cfg(unix)]
fn authenticated_client(
    input: &CliInput,
) -> Result<
    (
        crate::discovery::SelectedEndpoint,
        LocalControlClient,
        application_contract::LocalControlHandshake,
    ),
    DxbotError,
> {
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
    input: &mut CliInput,
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

fn render_operation(
    result: &OperationResult,
    command_key: &str,
    format: OutputFormat,
) -> CliOutput {
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
        OutputFormat::Jsonl => CliOutput::success(
            renderer.render_jsonl(&[StreamEvent::Terminal(Box::new(result.clone()))]),
        ),
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

fn decorate_output(mut output: CliOutput, enabled: bool) -> CliOutput {
    if !enabled {
        return output;
    }
    if !output.stdout.is_empty() {
        output.stdout = color_first_line(&output.stdout, "32");
    }
    if !output.stderr.is_empty() {
        output.stderr = color_first_line(&output.stderr, "31");
    }
    output
}

fn color_first_line(value: &str, ansi_code: &str) -> String {
    match value.split_once('\n') {
        Some((first, rest)) => format!("\x1b[{ansi_code}m{first}\x1b[0m\n{rest}"),
        None => format!("\x1b[{ansi_code}m{value}\x1b[0m"),
    }
}

fn emit_cli_output(output: &CliOutput) -> i32 {
    if !output.stdout.is_empty()
        && std::io::stdout()
            .write_all(output.stdout.as_bytes())
            .is_err()
    {
        return ErrorCode::Interrupted.exit_code();
    }
    if !output.stderr.is_empty()
        && std::io::stderr()
            .write_all(output.stderr.as_bytes())
            .is_err()
    {
        return ErrorCode::Interrupted.exit_code();
    }
    output.exit_code
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
                validate_global_option_value(token, value)?;
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

/// Rejects malformed enumerated global option values (format/color) before any
/// offline or group-help short-circuit can silently accept them. Value-bearing
/// options whose grammar is command-scoped are validated later during binding.
fn validate_global_option_value(flag: &str, value: &str) -> Result<(), DxbotError> {
    match flag {
        "--format" => matches!(value, "human" | "json" | "jsonl")
            .then_some(())
            .ok_or_else(|| usage_error(format!("invalid --format: '{value}'"))),
        "--color" => matches!(value, "auto" | "always" | "never")
            .then_some(())
            .ok_or_else(|| usage_error(format!("invalid --color: '{value}'"))),
        _ => Ok(()),
    }
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

fn requested_color(args: &[String]) -> ColorMode {
    args.windows(2)
        .find_map(|window| {
            if window[0] == "--color" {
                match window[1].as_str() {
                    "auto" => Some(ColorMode::Auto),
                    "always" => Some(ColorMode::Always),
                    "never" => Some(ColorMode::Never),
                    _ => None,
                }
            } else {
                None
            }
        })
        .unwrap_or(ColorMode::Auto)
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

fn diagnostic_page(input: &CliInput, diagnostics: Vec<Value>) -> Result<Value, DxbotError> {
    let section = local_string(input, "section")?;
    let mut diagnostics = diagnostics
        .into_iter()
        .filter(|item| {
            section
                .as_deref()
                .is_none_or(|wanted| item.get("section").and_then(Value::as_str) == Some(wanted))
        })
        .collect::<Vec<_>>();
    diagnostics.sort_by(|left, right| {
        left.get("section")
            .and_then(Value::as_str)
            .cmp(&right.get("section").and_then(Value::as_str))
    });
    if let Some(section) = section.as_deref()
        && diagnostics.is_empty()
    {
        return Err(input_error(format!(
            "unknown runtime doctor section: {section}; expected control, journal, or provider"
        )));
    }

    let page_size = match local_string(input, "page_size")? {
        Some(value) => value
            .parse::<usize>()
            .map_err(|_| input_error("runtime doctor page_size must be an integer"))?,
        None => DEFAULT_DIAGNOSTIC_PAGE_SIZE,
    };
    if !(1..=MAX_DIAGNOSTIC_PAGE_SIZE).contains(&page_size) {
        return Err(input_error(format!(
            "runtime doctor page_size must be in 1..={MAX_DIAGNOSTIC_PAGE_SIZE}"
        )));
    }
    let offset = match local_string(input, "cursor")? {
        Some(cursor) => {
            let encoded = cursor
                .strip_prefix(DIAGNOSTIC_CURSOR_PREFIX)
                .ok_or_else(|| input_error("invalid runtime doctor cursor"))?;
            encoded
                .parse::<usize>()
                .map_err(|_| input_error("invalid runtime doctor cursor"))?
        }
        None => 0,
    };
    if offset > diagnostics.len() {
        return Err(input_error(
            "runtime doctor cursor is beyond the result set",
        ));
    }
    let end = offset.saturating_add(page_size).min(diagnostics.len());
    let has_more = end < diagnostics.len();
    let next_cursor = has_more.then(|| format!("{DIAGNOSTIC_CURSOR_PREFIX}{end:019}"));
    Ok(json!({
        "items": diagnostics[offset..end].to_vec(),
        "next_cursor": next_cursor,
        "has_more": has_more,
    }))
}

fn local_bool(input: &CliInput, key: &str) -> Result<bool, DxbotError> {
    match input.fields.get(key) {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(Value::String(value)) if value == "true" => Ok(true),
        Some(Value::String(value)) if value == "false" => Ok(false),
        Some(_) => Err(input_error(format!("--{key} expects a boolean flag"))),
    }
}

fn local_string(input: &CliInput, key: &str) -> Result<Option<String>, DxbotError> {
    match input.fields.get(key) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(input_error(format!("--{key} expects one value"))),
    }
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
    let path = cli_path_tokens(metadata.command_key).join(" ");
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
        ClientError::Remote(error) => *error,
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
        ClientError::SimulatedCrash(point) => internal_error(format!(
            "simulated crash escaped production path: {point:?}"
        )),
    }
}

fn owner_unavailable(command_key: &str, kind: &str) -> DxbotError {
    error_with(
        ErrorCode::InternalInvariant,
        ErrorCategory::Internal,
        format!("registry command {command_key} ({kind}) has no connected production owner"),
    )
}

fn partial_error(message: impl Into<String>, resume_cursor: Option<String>) -> DxbotError {
    let mut error = error_with(ErrorCode::PartialOrResync, ErrorCategory::Recovery, message);
    error.resume_cursor = resume_cursor;
    error.retryable = true;
    error
}

fn usage_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::Usage, ErrorCategory::Input, message)
}

fn input_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::InvalidInput, ErrorCategory::Input, message)
}

fn local_error(message: impl Into<String>) -> DxbotError {
    error_with(
        ErrorCode::StorageOrCorruption,
        ErrorCategory::Local,
        message,
    )
}

fn storage_error(message: impl Into<String>) -> DxbotError {
    error_with(
        ErrorCode::StorageOrCorruption,
        ErrorCategory::Integrity,
        message,
    )
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
    let mut error = error_with(ErrorCode::Timeout, ErrorCategory::Availability, message);
    error.retryable = true;
    error
}

fn interrupted_error(message: impl Into<String>) -> DxbotError {
    error_with(ErrorCode::Interrupted, ErrorCategory::Local, message)
}

fn internal_error(message: impl Into<String>) -> DxbotError {
    error_with(
        ErrorCode::InternalInvariant,
        ErrorCategory::Internal,
        message,
    )
}

fn error_with(code: ErrorCode, category: ErrorCategory, message: impl Into<String>) -> DxbotError {
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
        assert_eq!(value["supported_schema_versions"][0], "v2");
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

    #[test]
    fn stream_json_is_classified_before_unbounded_buffering() {
        let prepared = prepare_invocation(&args(&[
            "task", "watch", "--task", "task-a", "--format", "json",
        ]))
        .expect("invocation parses");
        match prepared {
            PreparedInvocation::Command { input, kind, .. } => {
                assert_eq!(kind, "S");
                assert_eq!(input.global_options.format, OutputFormat::Json);
            }
            PreparedInvocation::Offline(_) => panic!("expected command"),
        }
    }

    #[test]
    fn watch_poll_is_strictly_inside_transport_timeout() {
        assert!(watch_poll_timeout(Duration::from_secs(30)) <= WATCH_POLL_MAX);
        assert!(watch_poll_timeout(Duration::from_millis(10)) < Duration::from_millis(10));
    }

    #[test]
    fn color_always_is_explicit_for_human_output() {
        let output = execute(&args(&["--color", "always", "--version"]));
        assert!(output.stdout.starts_with("\u{1b}[32m"));
    }

    #[test]
    fn runtime_doctor_fields_drive_section_and_cursor_page() {
        let first_input = parse_bound_input(&args(&["runtime-doctor", "--page-size", "1"]))
            .expect("doctor input");
        let diagnostics = vec![
            json!({"section": "provider", "status": "ready"}),
            json!({"section": "control", "status": "ready"}),
        ];
        let first = diagnostic_page(&first_input, diagnostics.clone()).expect("first page");
        assert_eq!(first["items"].as_array().expect("items").len(), 1);
        assert_eq!(first["items"][0]["section"], "control");
        assert_eq!(first["next_cursor"], "d0000000000000000001");

        let second_input = parse_bound_input(&args(&[
            "runtime-doctor",
            "--page-size",
            "1",
            "--cursor",
            "d0000000000000000001",
        ]))
        .expect("doctor cursor input");
        let second = diagnostic_page(&second_input, diagnostics).expect("second page");
        assert_eq!(second["items"][0]["section"], "provider");
        assert_eq!(second["has_more"], false);
    }

    #[test]
    fn runtime_doctor_section_filters_and_unknown_fails_closed() {
        let provider = parse_bound_input(&args(&["runtime-doctor", "--section", "provider"]))
            .expect("provider input");
        let diagnostics = vec![
            json!({"section": "control"}),
            json!({"section": "provider"}),
        ];
        let page = diagnostic_page(&provider, diagnostics.clone()).expect("provider page");
        assert_eq!(page["items"].as_array().expect("items").len(), 1);
        assert_eq!(page["items"][0]["section"], "provider");

        let unknown = parse_bound_input(&args(&["runtime-doctor", "--section", "secrets"]))
            .expect("unknown section parses generically");
        assert!(diagnostic_page(&unknown, diagnostics).is_err());
    }

    #[test]
    fn machine_output_never_contains_ansi_even_when_color_is_always() {
        let output = execute(&args(&[
            "--format",
            "json",
            "--color",
            "always",
            "--version",
        ]));
        assert!(!output.stdout.contains("\u{1b}["));
        let _: Value = serde_json::from_str(output.stdout.trim()).expect("valid json");
    }
}
