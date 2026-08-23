//! Multi-process journal probe used by `tests/journal.rs`.
//!
//! Usage: `journal-helper <output_base> <command_id>`
//!
//! Attempts an OS-atomic exclusive create of `<output_base>/<command_id>.jsonl`
//! (the same layout the local journal uses). Exits `0` if this process became
//! the exclusive creator, `2` if the file already exists (another process
//! owns it), and `3` on any other error.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let _program = args.next();
    let Some(base) = args.next() else {
        eprintln!("usage: journal-helper <output_base> <command_id>");
        return ExitCode::from(3);
    };
    let Some(command_id) = args.next() else {
        eprintln!("usage: journal-helper <output_base> <command_id>");
        return ExitCode::from(3);
    };

    let path = PathBuf::from(base)
        .join(command_id)
        .with_extension("jsonl");
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
            let _ = file.write_all(b"winner\n");
            let _ = file.sync_all();
            ExitCode::from(0)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => ExitCode::from(2),
        Err(_) => ExitCode::from(3),
    }
}