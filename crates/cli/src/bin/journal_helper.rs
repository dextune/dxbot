//! Multi-process probe for the real [`cli::journal::LocalJournal`] ownership path.
//!
//! Usage: `journal-helper <base> <command-id> [hold-ms]`.

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use cli::journal::{JournalError, LocalJournal};
use dxbot_core::types::{
    CommandId, IdempotencyKey, InstanceId, JournalRecord, JournalState, OperationId, PrincipalRef,
    RequestDigest,
};

fn main() -> ExitCode {
    let mut args = std::env::args();
    let _program = args.next();
    let Some(base) = args.next() else {
        return ExitCode::from(3);
    };
    let Some(command) = args.next() else {
        return ExitCode::from(3);
    };
    let hold_ms = args
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);

    let instance = InstanceId("multi-process".to_owned());
    let mut journal = match LocalJournal::open(instance.clone(), Path::new(&base)) {
        Ok(journal) => journal,
        Err(_) => return ExitCode::from(3),
    };
    let record = JournalRecord {
        state: JournalState::Prepared,
        instance_id: instance,
        command_id: CommandId(command.clone()),
        operation_id: OperationId(format!("operation-{command}")),
        idempotency_key: IdempotencyKey {
            principal_ref: PrincipalRef("probe".to_owned()),
            key_digest: format!("key-{command}"),
            expires_at: 0,
        },
        request_digest: RequestDigest(format!("request-{command}")),
        sequence: 0,
        previous_digest: String::new(),
        record_digest: String::new(),
    };

    match journal.append_prepared(&record) {
        Ok(()) => {
            if hold_ms > 0 {
                std::thread::sleep(Duration::from_millis(hold_ms));
            }
            ExitCode::SUCCESS
        }
        Err(JournalError::LockBusy(_) | JournalError::AlreadyExists(_)) => ExitCode::from(2),
        Err(_) => ExitCode::from(3),
    }
}
