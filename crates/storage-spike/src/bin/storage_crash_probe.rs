use std::error::Error;
use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;

use storage_spike::{CrashPoint, OperationEffect, OperationRequest, ReferenceStore};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    let path = PathBuf::from(arguments.next().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "missing database path")
    })?);
    let mode = arguments
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing crash mode"))?;

    let crash_point = match mode.as_os_str() {
        value if value == OsStr::new("before-commit") => CrashPoint::BeforeCommit,
        value if value == OsStr::new("after-commit") => CrashPoint::AfterCommitBeforeResponse,
        _ => {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "unknown crash mode").into());
        }
    };

    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let request = OperationRequest {
        principal_ref: "principal-a",
        idempotency_key_principal_ref: "principal-a",
        idempotency_key_digest: "key-a",
        command_id: "command-a",
        request_digest: "request-a",
        new_operation_id: "operation-a",
        key_expires_at: 100,
        now: 10,
    };
    let effect = OperationEffect {
        aggregate_id: "bot-1",
        state_value: "state-v1",
        event_payload: "event-v1",
        outbox_payload: "outbox-v1",
        audit_payload: "audit-v1",
        resolved_binding_digest: "binding-v1",
        result_ref: "result-1",
    };

    let _outcome = store.submit_with_crash_point(&request, &effect, crash_point)?;
    Err(io::Error::other("configured crash point did not terminate process").into())
}
