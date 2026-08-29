//! `dxb` process entrypoint.
//!
//! All public command semantics live in `cli::runner`; this file only converts
//! OS argv, handles the private Runtime Host process mode, and delegates bounded
//! or incremental output to the production runner.

use std::ffi::OsStr;
use std::io::Write;

fn main() {
    let raw_args: Vec<_> = std::env::args_os().skip(1).collect();
    if raw_args
        .first()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value == "__runtime-host")
    {
        let result = cli::run_runtime_host_process(&raw_args[1..]);
        match result {
            Ok(()) => std::process::exit(0),
            Err(message) => {
                let _ = writeln!(std::io::stderr(), "Runtime Host error: {message}");
                std::process::exit(10);
            }
        }
    }

    let mut args = Vec::with_capacity(raw_args.len());
    for argument in raw_args {
        match argument.into_string() {
            Ok(argument) => args.push(argument),
            Err(argument) => exit_invalid_unicode(&argument),
        }
    }
    std::process::exit(cli::run_process(&args));
}

fn exit_invalid_unicode(argument: &OsStr) -> ! {
    let _ = writeln!(
        std::io::stderr(),
        "error Usage: CLI argument is not valid UTF-8: {:?}",
        argument
    );
    std::process::exit(2)
}
