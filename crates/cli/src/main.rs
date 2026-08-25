use std::io::IsTerminal;
use std::process::ExitCode;

use cli::{commands::CoreCommands, confirmation::Confirmation, directive::DirectiveController};
use dxbot_core::{
    error::{DxbotError, ErrorCategory, ErrorCode},
    types::{ContentSource, InstanceId, OutputFormat, PrincipalRef},
};

fn parse_format(value: &str) -> Option<OutputFormat> {
    match value {
        "human" => Some(OutputFormat::Human),
        "json" => Some(OutputFormat::Json),
        "jsonl" => Some(OutputFormat::Jsonl),
        _ => None,
    }
}

fn error_code(code: ErrorCode) -> ExitCode {
    ExitCode::from(code.exit_code() as u8)
}

fn print_error(err: &DxbotError, format: OutputFormat) -> ExitCode {
    let code = err.code;
    match format {
        OutputFormat::Json | OutputFormat::Jsonl => {
            let _ = serde_json::to_writer(std::io::stdout().lock(), err);
            println!();
        }
        OutputFormat::Human => {
            eprintln!("error: {}", err.message);
        }
    }
    error_code(code)
}

fn local_error(code: ErrorCode, category: ErrorCategory, message: impl Into<String>) -> DxbotError {
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

fn usage_error(message: impl Into<String>, format: OutputFormat) -> ExitCode {
    print_error(
        &local_error(ErrorCode::Usage, ErrorCategory::Input, message),
        format,
    )
}

/// Fail closed for a command whose local projection exists but whose
/// authenticated ControlClient submission path is not connected yet.
/// No durable operation is created and no success-like payload is printed.
fn not_wired(command: &str, format: OutputFormat) -> ExitCode {
    print_error(
        &local_error(
            ErrorCode::RuntimeUnavailable,
            ErrorCategory::Availability,
            format!(
                "{command} is not connected to an authenticated Runtime control endpoint; no operation was created"
            ),
        ),
        format,
    )
}

fn require_value(args: &[String], index: &mut usize, option: &str) -> Result<(), String> {
    *index += 1;
    if args.get(*index).is_none() {
        return Err(format!("{option} requires a value"));
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print!("{}", cli::discovery::Discovery::show_help());
        return ExitCode::SUCCESS;
    }

    let mut format = OutputFormat::Human;
    let mut yes_flag = false;
    let is_tty = std::io::stdin().is_terminal();
    let mut version_requested = false;
    let mut cmd_start = None;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{}", cli::discovery::Discovery::show_help());
                return ExitCode::SUCCESS;
            }
            "--version" => version_requested = true,
            "--format" => {
                if let Err(message) = require_value(&args, &mut i, "--format") {
                    return usage_error(message, format);
                }
                let value = &args[i];
                let Some(parsed) = parse_format(value) else {
                    return usage_error(
                        format!("unsupported --format value: {value}"),
                        format,
                    );
                };
                format = parsed;
            }
            "--profile" | "--instance" | "--color" | "--wait" | "--timeout" => {
                let option = args[i].clone();
                if let Err(message) = require_value(&args, &mut i, &option) {
                    return usage_error(message, format);
                }
            }
            "-y" | "--yes" => yes_flag = true,
            value if value.starts_with('-') => {
                return usage_error(format!("unknown global option: {value}"), format);
            }
            _ => {
                cmd_start = Some(i);
                break;
            }
        }
        i += 1;
    }

    if version_requested {
        return show_version(format);
    }

    let Some(cmd_start) = cmd_start else {
        return usage_error("missing command group", format);
    };
    let group = &args[cmd_start];
    let sub = args.get(cmd_start + 1).map(String::as_str);
    let cmd_args: &[String] = if cmd_start + 2 < args.len() {
        &args[cmd_start + 2..]
    } else {
        &[]
    };

    let commands = CoreCommands::at(
        InstanceId("default".to_owned()),
        PrincipalRef("cli".to_owned()),
    );
    let directive = DirectiveController::new();

    match group.as_str() {
        "version" => show_version(format),
        "runtime" => dispatch_unwired_group(
            "runtime",
            sub,
            &["start", "status", "stop", "doctor"],
            format,
        ),
        "bot" => dispatch_bot(sub, cmd_args, &commands, format, yes_flag, is_tty),
        "conversation" => dispatch_conversation(sub, cmd_args, &commands, format),
        "thread" => dispatch_unwired_group(
            "thread",
            sub,
            &["create", "list", "show", "send", "history", "branch"],
            format,
        ),
        "task" => dispatch_task(sub, cmd_args, &commands, &directive, format),
        "project" => dispatch_unwired_group(
            "project",
            sub,
            &["create", "list", "show", "archive", "restore", "member"],
            format,
        ),
        "channel" => dispatch_unwired_group(
            "channel",
            sub,
            &["create", "list", "show", "member", "send", "history"],
            format,
        ),
        "memory" => dispatch_unwired_group(
            "memory",
            sub,
            &["get", "search", "history", "propose", "promote"],
            format,
        ),
        "approval" => dispatch_unwired_group(
            "approval",
            sub,
            &["list", "show", "approve", "deny"],
            format,
        ),
        "provider" => dispatch_unwired_group("provider", sub, &["list", "show"], format),
        "operation" => {
            dispatch_unwired_group("operation", sub, &["show", "reconcile"], format)
        }
        "process" => dispatch_unwired_group("process", sub, &["show", "watch"], format),
        _ => usage_error(format!("unknown command group: {group}"), format),
    }
}

fn show_version(format: OutputFormat) -> ExitCode {
    let info = cli::discovery::Discovery::show_version();
    match format {
        OutputFormat::Human => println!("dxb {}", info.client_version),
        _ => match serde_json::to_string(&info) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                return print_error(
                    &local_error(
                        ErrorCode::InternalInvariant,
                        ErrorCategory::Internal,
                        format!("failed to encode version information: {error}"),
                    ),
                    format,
                );
            }
        },
    }
    ExitCode::SUCCESS
}

fn dispatch_unwired_group(
    group: &str,
    sub: Option<&str>,
    allowed: &[&str],
    format: OutputFormat,
) -> ExitCode {
    match sub {
        Some(command) if allowed.contains(&command) => {
            not_wired(&format!("{group} {command}"), format)
        }
        Some(command) => usage_error(
            format!("unknown {group} subcommand: {command}"),
            format,
        ),
        None => usage_error(format!("{group} requires a subcommand"), format),
    }
}

fn dispatch_bot(
    sub: Option<&str>,
    args: &[String],
    commands: &CoreCommands,
    format: OutputFormat,
    yes_flag: bool,
    is_tty: bool,
) -> ExitCode {
    match sub {
        Some("create") => {
            let name = args.first().map(String::as_str).unwrap_or_default();
            match commands.bot_create(name, &std::collections::HashMap::new()) {
                Ok(_) => not_wired("bot create", format),
                Err(error) => print_error(&error.to_dxbot_error(), format),
            }
        }
        Some("archive") => {
            let Some(target) = args.first().map(String::as_str) else {
                return usage_error("bot archive requires a target", format);
            };
            match Confirmation::require_destructive("archive bot", target, is_tty, yes_flag) {
                Ok(true) => not_wired("bot archive", format),
                Ok(false) => {
                    if format == OutputFormat::Human {
                        println!("Archive cancelled.");
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => print_error(&error.to_dxbot_error(), format),
            }
        }
        Some(command) if ["list", "show", "activate", "deactivate", "restore"].contains(&command) => {
            not_wired(&format!("bot {command}"), format)
        }
        Some(command) => usage_error(format!("unknown bot subcommand: {command}"), format),
        None => usage_error("bot requires a subcommand", format),
    }
}

fn dispatch_conversation(
    sub: Option<&str>,
    args: &[String],
    commands: &CoreCommands,
    format: OutputFormat,
) -> ExitCode {
    match sub {
        Some("send") => {
            let selector = args.first().map(String::as_str).unwrap_or_default();
            let content = if args.iter().any(|argument| argument == "--stdin") {
                ContentSource::Stdin
            } else {
                ContentSource::Text {
                    value: args.get(1).cloned().unwrap_or_default(),
                }
            };
            match commands.conversation_send(selector, &content) {
                Ok(_) => not_wired("conversation send", format),
                Err(error) => print_error(&error.to_dxbot_error(), format),
            }
        }
        Some(command) if ["show", "history"].contains(&command) => {
            not_wired(&format!("conversation {command}"), format)
        }
        Some(command) => usage_error(
            format!("unknown conversation subcommand: {command}"),
            format,
        ),
        None => usage_error("conversation requires a subcommand", format),
    }
}

fn dispatch_task(
    sub: Option<&str>,
    args: &[String],
    commands: &CoreCommands,
    directive: &DirectiveController,
    format: OutputFormat,
) -> ExitCode {
    match sub {
        Some("submit") => {
            let owner = args.first().map(String::as_str).unwrap_or_default();
            let content = ContentSource::Text {
                value: args.get(1).cloned().unwrap_or_default(),
            };
            match commands.task_submit(owner, &content, &cli::TaskOptions::default()) {
                Ok(_) => not_wired("task submit", format),
                Err(error) => print_error(&error.to_dxbot_error(), format),
            }
        }
        Some("show") | Some("result") => {
            let selector = args.first().map(String::as_str).unwrap_or_default();
            let projected = if sub == Some("show") {
                commands.task_show(selector)
            } else {
                commands.task_result(selector)
            };
            match projected {
                Ok(_) => not_wired(&format!("task {}", sub.unwrap_or_default()), format),
                Err(error) => print_error(&error.to_dxbot_error(), format),
            }
        }
        Some("cancel") => project_directive(
            directive.cancel_task(
                args.first().map(String::as_str).unwrap_or_default(),
                None,
                None,
                None,
            ),
            "task cancel",
            format,
        ),
        Some("suspend") => project_directive(
            directive.suspend_task(
                args.first().map(String::as_str).unwrap_or_default(),
                None,
                None,
                None,
            ),
            "task suspend",
            format,
        ),
        Some("resume") => project_directive(
            directive.resume_task(
                args.first().map(String::as_str).unwrap_or_default(),
                None,
            ),
            "task resume",
            format,
        ),
        Some("redirect") => {
            let replacement = ContentSource::Text {
                value: args.get(1).cloned().unwrap_or_default(),
            };
            project_directive(
                directive.redirect_task(
                    args.first().map(String::as_str).unwrap_or_default(),
                    None,
                    &replacement,
                ),
                "task redirect",
                format,
            )
        }
        Some("list") | Some("watch") => {
            not_wired(&format!("task {}", sub.unwrap_or_default()), format)
        }
        Some(command) => usage_error(format!("unknown task subcommand: {command}"), format),
        None => usage_error("task requires a subcommand", format),
    }
}

fn project_directive(
    projection: Result<dxbot_core::types::CommandPayload, cli::directive::DirectiveError>,
    command: &str,
    format: OutputFormat,
) -> ExitCode {
    match projection {
        Ok(_) => not_wired(command, format),
        Err(error) => print_error(&directive_error_to_dxbot(&error), format),
    }
}

fn directive_error_to_dxbot(error: &cli::directive::DirectiveError) -> DxbotError {
    use cli::directive::DirectiveError;
    match error {
        DirectiveError::InvalidSelector { selector } => local_error(
            ErrorCode::InvalidInput,
            ErrorCategory::Input,
            format!("invalid task selector: {selector}"),
        ),
        DirectiveError::ReplacementRequired => local_error(
            ErrorCode::InvalidInput,
            ErrorCategory::Input,
            "redirect requires replacement content",
        ),
        DirectiveError::ObserverLost => local_error(
            ErrorCode::InternalInvariant,
            ErrorCategory::Internal,
            "directive observer lost",
        ),
    }
}
