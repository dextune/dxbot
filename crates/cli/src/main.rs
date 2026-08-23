use std::io::IsTerminal;
use std::process::ExitCode;

use cli::{
    commands::CoreCommands,
    confirmation::Confirmation,
    directive::DirectiveController,
    discovery::Discovery,
    interactive::InteractiveJourney,
    selector::SelectorResolver,
};
use dxbot_core::{
    error::DxbotError,
    types::{ContentSource, InstanceId, OutputFormat, PrincipalRef},
};

// ── Helpers ──

fn parse_format(value: &str) -> OutputFormat {
    match value {
        "json" => OutputFormat::Json,
        "jsonl" => OutputFormat::Jsonl,
        _ => OutputFormat::Human,
    }
}

fn error_code(code: dxbot_core::error::ErrorCode) -> ExitCode {
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

fn human_action(msg: &str) {
    println!("{msg}");
}

fn json_action(result: &serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(result).unwrap_or_default());
}

// ── Main dispatcher ──

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print!("{}", Discovery::show_help());
        return ExitCode::SUCCESS;
    }

    // Parse global options and command
    let mut format = OutputFormat::Human;
    let mut yes_flag = false;
    let is_tty = std::io::stdout().is_terminal();
    let mut cmd_start = 0;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{}", Discovery::show_help());
                return ExitCode::SUCCESS;
            }
            "--version" => {
                return show_version(format);
            }
            "--format" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    format = parse_format(v);
                }
            }
            "-y" | "--yes" => yes_flag = true,
            s if !s.starts_with('-') => {
                cmd_start = i;
                break;
            }
            _ => {}
        }
        i += 1;
    }

    if cmd_start >= args.len() {
        print!("{}", Discovery::show_help());
        return ExitCode::SUCCESS;
    }

    let group = &args[cmd_start];
    let sub = args.get(cmd_start + 1).map(String::as_str);
    let cmd_args: &[String] = if cmd_start + 2 < args.len() {
        &args[cmd_start + 2..]
    } else {
        &[]
    };

    let default_instance = InstanceId("default".into());
    let default_principal = PrincipalRef("cli".into());
    let discovery = Discovery::new();
    let commands = CoreCommands::at(default_instance, default_principal);
    let interactive = InteractiveJourney::default();
    let selector = SelectorResolver::new();
    let directive = DirectiveController::new();

    match group.as_str() {
        "version" => show_version(format),

        "runtime" => dispatch_runtime(sub, cmd_args, &discovery, format),

        "bot" => dispatch_bot(sub, cmd_args, &commands, &interactive, &selector, format, yes_flag, is_tty),

        "conversation" => dispatch_conversation(sub, cmd_args, &commands, &interactive, &selector, format),

        "thread" => dispatch_thread(sub, cmd_args, format),

        "task" => dispatch_task(sub, cmd_args, &commands, &interactive, &selector, &directive, format),

        "project" => dispatch_project(sub, cmd_args, format),

        "channel" => dispatch_channel(sub, cmd_args, format),

        "memory" => dispatch_memory(sub, cmd_args, format),

        "approval" => dispatch_approval(sub, cmd_args, format),

        "provider" => dispatch_provider(sub, cmd_args, format),

        "operation" => dispatch_operation(sub, cmd_args, format),

        "process" => dispatch_process(sub, cmd_args, format),

        _ => {
            if format == OutputFormat::Human {
                eprintln!("dxb: unknown command group '{group}'");
                eprintln!("Run 'dxb --help' for available commands.");
            }
            ExitCode::from(2)
        }
    }
}

// ── Version ──

fn show_version(format: OutputFormat) -> ExitCode {
    let info = Discovery::show_version();
    match format {
        OutputFormat::Human => {
            println!("dxb {}", info.client_version);
        }
        _ => {
            println!("{}", serde_json::to_string(&info).unwrap_or_default());
        }
    }
    ExitCode::SUCCESS
}

// ── Runtime ──

fn dispatch_runtime(
    sub: Option<&str>,
    args: &[String],
    discovery: &Discovery,
    format: OutputFormat,
) -> ExitCode {
    match sub {
        Some("start") => {
            if args.iter().any(|a| a == "--help" || a == "-h") {
                println!("dxb runtime start [--ready-at <process|storage|runtime|control>] [--timeout <duration>]");
                println!();
                println!("Bootstraps the first Runtime Instance when none exist, or starts an existing Instance.");
                println!("Ready-at stages: process (default), storage, runtime, control.");
                return ExitCode::SUCCESS;
            }
            match discovery.first_run_bootstrap() {
                Ok(instance_id) => {
                    human_action(&format!("Runtime instance '{}' started.", instance_id.0));
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&e.to_dxbot_error(), format),
            }
        }
        Some("status") => {
            human_action("Runtime status: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        Some("stop") => {
            human_action("Runtime stop: graceful shutdown requested.");
            ExitCode::SUCCESS
        }
        Some("doctor") => {
            match discovery.first_run_bootstrap() {
                Ok(instance_id) => {
                    match discovery.doctor_provider(&instance_id) {
                        Ok(diag) => {
                            human_action(&format!(
                                "Provider '{}': status={}, available={}",
                                diag.provider_id, diag.status, diag.available
                            ));
                            ExitCode::SUCCESS
                        }
                        Err(e) => print_error(&e.to_dxbot_error(), format),
                    }
                }
                Err(e) => print_error(&e.to_dxbot_error(), format),
            }
        }
        _ => {
            eprintln!("dxb runtime: unknown subcommand. Try 'start', 'status', 'stop', or 'doctor'.");
            ExitCode::from(2)
        }
    }
}

// ── Bot ──

#[allow(clippy::too_many_arguments)]
fn dispatch_bot(
    sub: Option<&str>,
    args: &[String],
    commands: &CoreCommands,
    _interactive: &InteractiveJourney,
    _selector: &SelectorResolver,
    format: OutputFormat,
    yes_flag: bool,
    is_tty: bool,
) -> ExitCode {
    match sub {
        Some("create") => {
            let name = args.first().cloned().unwrap_or_default();
            if name.is_empty() {
                return print_error(
                    &DxbotError {
                        code: dxbot_core::error::ErrorCode::Usage,
                        category: dxbot_core::error::ErrorCategory::Input,
                        message: "bot create requires a name".into(),
                        retryable: false,
                        operation_ref: None,
                        target_refs: vec![],
                        field_violations: vec![],
                        current_revision: None,
                        current_generation: None,
                        resume_cursor: None,
                        next_actions: vec![],
                    },
                    format,
                );
            }
            match commands.bot_create(&name, &std::collections::HashMap::new()) {
                Ok(payload) => {
                    json_action(&serde_json::to_value(&payload).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&e.to_dxbot_error(), format),
            }
        }
        Some("list") => {
            human_action("Bots: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        Some("show") => {
            human_action("Bot show: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        Some("activate") | Some("deactivate") => {
            human_action("Lifecycle change: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        Some("archive") => {
            let target = args.first().map(|s| s.as_str()).unwrap_or("unknown");
            match Confirmation::require_destructive("archive bot", target, is_tty, yes_flag) {
                Ok(true) => {
                    human_action(&format!("Bot '{target}' archived."));
                    ExitCode::SUCCESS
                }
                Ok(false) => {
                    human_action("Archive cancelled.");
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&e.to_dxbot_error(), format),
            }
        }
        Some("restore") => {
            human_action("Bot restore: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("dxb bot: unknown subcommand. Try 'create', 'list', 'show', 'activate', 'deactivate', 'archive', or 'restore'.");
            ExitCode::from(2)
        }
    }
}

// ── Conversation ──

fn dispatch_conversation(
    sub: Option<&str>,
    args: &[String],
    commands: &CoreCommands,
    _interactive: &InteractiveJourney,
    _selector: &SelectorResolver,
    format: OutputFormat,
) -> ExitCode {
    match sub {
        Some("send") => {
            let bot_selector = args.first().cloned().unwrap_or_default();
            let content = if args.iter().any(|a| a == "--stdin") {
                ContentSource::Stdin
            } else {
                ContentSource::Text {
                    value: args.get(1).cloned().unwrap_or_default(),
                }
            };
            match commands.conversation_send(&bot_selector, &content) {
                Ok(payload) => {
                    json_action(&serde_json::to_value(&payload).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&e.to_dxbot_error(), format),
            }
        }
        Some("show") => {
            human_action("Conversation show: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        Some("history") => {
            human_action("Conversation history: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("dxb conversation: unknown subcommand. Try 'show', 'send', or 'history'.");
            ExitCode::from(2)
        }
    }
}

// ── Thread ──

fn dispatch_thread(
    sub: Option<&str>,
    _args: &[String],
    _format: OutputFormat,
) -> ExitCode {
    match sub {
        Some("create") | Some("list") | Some("show") | Some("send") | Some("history") | Some("branch") => {
            human_action("Thread operations: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("dxb thread: unknown subcommand. Try 'create', 'list', 'show', 'send', 'history', or 'branch'.");
            ExitCode::from(2)
        }
    }
}

// ── Task ──

fn dispatch_task(
    sub: Option<&str>,
    args: &[String],
    commands: &CoreCommands,
    _interactive: &InteractiveJourney,
    _selector: &SelectorResolver,
    directive: &DirectiveController,
    format: OutputFormat,
) -> ExitCode {
    match sub {
        Some("submit") => {
            let owner = args.first().cloned().unwrap_or_default();
            let content = ContentSource::Text {
                value: args.get(1).cloned().unwrap_or_default(),
            };
            let options = cli::TaskOptions::default();
            match commands.task_submit(&owner, &content, &options) {
                Ok(payload) => {
                    json_action(&serde_json::to_value(&payload).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&e.to_dxbot_error(), format),
            }
        }
        Some("list") => {
            human_action("Tasks: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        Some("show") => {
            let selector = args.first().cloned().unwrap_or_default();
            match commands.task_show(&selector) {
                Ok(payload) => {
                    json_action(&serde_json::to_value(&payload).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&e.to_dxbot_error(), format),
            }
        }
        Some("result") => {
            let selector = args.first().cloned().unwrap_or_default();
            match commands.task_result(&selector) {
                Ok(payload) => {
                    json_action(&serde_json::to_value(&payload).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&e.to_dxbot_error(), format),
            }
        }
        Some("cancel") => {
            let selector = args.first().cloned().unwrap_or_default();
            match directive.cancel_task(&selector, None, None, None) {
                Ok(payload) => {
                    json_action(&serde_json::to_value(&payload).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&directive_error_to_dxbot(&e), format),
            }
        }
        Some("suspend") => {
            let selector = args.first().cloned().unwrap_or_default();
            match directive.suspend_task(&selector, None, None, None) {
                Ok(payload) => {
                    json_action(&serde_json::to_value(&payload).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&directive_error_to_dxbot(&e), format),
            }
        }
        Some("resume") => {
            let selector = args.first().cloned().unwrap_or_default();
            match directive.resume_task(&selector, None) {
                Ok(payload) => {
                    json_action(&serde_json::to_value(&payload).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&directive_error_to_dxbot(&e), format),
            }
        }
        Some("redirect") => {
            let selector = args.first().cloned().unwrap_or_default();
            let replacement = ContentSource::Text {
                value: args.get(1).cloned().unwrap_or_default(),
            };
            match directive.redirect_task(&selector, None, &replacement) {
                Ok(payload) => {
                    json_action(&serde_json::to_value(&payload).unwrap_or_default());
                    ExitCode::SUCCESS
                }
                Err(e) => print_error(&directive_error_to_dxbot(&e), format),
            }
        }
        _ => {
            eprintln!("dxb task: unknown subcommand. Try 'submit', 'list', 'show', 'cancel', 'suspend', 'resume', 'redirect', or 'result'.");
            ExitCode::from(2)
        }
    }
}

fn directive_error_to_dxbot(err: &cli::directive::DirectiveError) -> DxbotError {
    use cli::directive::DirectiveError;
    match err {
        DirectiveError::InvalidSelector { selector } => DxbotError {
            code: dxbot_core::error::ErrorCode::InvalidInput,
            category: dxbot_core::error::ErrorCategory::Input,
            message: format!("invalid task selector: {selector}"),
            retryable: false,
            operation_ref: None,
            target_refs: vec![selector.clone()],
            field_violations: vec![],
            current_revision: None,
            current_generation: None,
            resume_cursor: None,
            next_actions: vec![],
        },
        DirectiveError::ReplacementRequired => DxbotError {
            code: dxbot_core::error::ErrorCode::InvalidInput,
            category: dxbot_core::error::ErrorCategory::Input,
            message: "redirect requires replacement content".into(),
            retryable: false,
            operation_ref: None,
            target_refs: vec![],
            field_violations: vec![],
            current_revision: None,
            current_generation: None,
            resume_cursor: None,
            next_actions: vec![],
        },
        DirectiveError::ObserverLost => DxbotError {
            code: dxbot_core::error::ErrorCode::InternalInvariant,
            category: dxbot_core::error::ErrorCategory::Internal,
            message: "directive observer lost".into(),
            retryable: false,
            operation_ref: None,
            target_refs: vec![],
            field_violations: vec![],
            current_revision: None,
            current_generation: None,
            resume_cursor: None,
            next_actions: vec![],
        },
    }
}

// ── Project ──

fn dispatch_project(
    sub: Option<&str>,
    _args: &[String],
    _format: OutputFormat,
) -> ExitCode {
    match sub {
        Some("create") | Some("list") | Some("show") | Some("archive") | Some("restore") => {
            human_action("Project operations: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        Some("member") => {
            human_action("Project member operations: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("dxb project: unknown subcommand. Try 'create', 'list', 'show', 'archive', 'restore', or 'member'.");
            ExitCode::from(2)
        }
    }
}

// ── Channel ──

fn dispatch_channel(
    sub: Option<&str>,
    _args: &[String],
    _format: OutputFormat,
) -> ExitCode {
    match sub {
        Some("create") | Some("list") | Some("show") => {
            human_action("Channel operations: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        Some("member") => {
            human_action("Channel member operations: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        Some("send") | Some("history") => {
            human_action("Channel conversation: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("dxb channel: unknown subcommand. Try 'create', 'list', 'show', 'member', 'send', or 'history'.");
            ExitCode::from(2)
        }
    }
}

// ── Memory ──

fn dispatch_memory(sub: Option<&str>, _args: &[String], _format: OutputFormat) -> ExitCode {
    match sub {
        Some("get") | Some("search") | Some("history") | Some("propose") | Some("promote") => {
            human_action("Memory operations: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("dxb memory: unknown subcommand. Try 'get', 'search', 'history', 'propose', or 'promote'.");
            ExitCode::from(2)
        }
    }
}

// ── Approval ──

fn dispatch_approval(sub: Option<&str>, _args: &[String], _format: OutputFormat) -> ExitCode {
    match sub {
        Some("list") | Some("show") | Some("approve") | Some("deny") => {
            human_action("Approval operations: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("dxb approval: unknown subcommand. Try 'list', 'show', 'approve', or 'deny'.");
            ExitCode::from(2)
        }
    }
}

// ── Provider ──

fn dispatch_provider(sub: Option<&str>, _args: &[String], _format: OutputFormat) -> ExitCode {
    match sub {
        Some("list") | Some("show") => {
            human_action("Provider operations: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("dxb provider: unknown subcommand. Try 'list' or 'show'.");
            ExitCode::from(2)
        }
    }
}

// ── Operation ──

fn dispatch_operation(sub: Option<&str>, _args: &[String], _format: OutputFormat) -> ExitCode {
    match sub {
        Some("show") | Some("reconcile") => {
            human_action("Operation operations: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("dxb operation: unknown subcommand. Try 'show' or 'reconcile'.");
            ExitCode::from(2)
        }
    }
}

// ── Process ──

fn dispatch_process(sub: Option<&str>, _args: &[String], _format: OutputFormat) -> ExitCode {
    match sub {
        Some("show") | Some("watch") => {
            human_action("Process operations: (requires live endpoint)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("dxb process: unknown subcommand. Try 'show' or 'watch'.");
            ExitCode::from(2)
        }
    }
}