//! Local journal: OS-locked single-writer ownership, hash chain, and bounded
//! retention.
//!
//! The journal persists `Prepared -> Dispatching -> Observed -> Terminal` (or
//! `Prepared -> Abandoned`) per command. Ownership is an advisory exclusive
//! file lock held by a live file descriptor, so process exit/crash releases it
//! in the kernel without stale-lock deletion heuristics.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use dxbot_core::types::*;
use fs2::FileExt;

#[path = "journal-sha256.rs"]
mod journal_sha256;
use journal_sha256::sha256_hex;

const MAX_SCAN_RECORDS: usize = 1_000_000;
const ZERO_HASH_HEX: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrunePolicy {
    /// A `Prepared` command is never capacity-pruned before this age.
    pub min_retention_seconds: u64,
    /// Desired upper bound for retained `Prepared` command files. The bound is
    /// best-effort while records are too fresh, locked, or integrity-invalid.
    pub max_prepared_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalError {
    Io(String),
    Corrupt(String),
    AlreadyExists(CommandId),
    NotFound(CommandId),
    LockBusy(CommandId),
    InvalidCommandId(CommandId),
    NotOwner(CommandId),
    ChainIntegrity(String),
    InvalidTransition {
        command_id: CommandId,
        from: JournalState,
        to: JournalState,
    },
    CapacityBoundExceeded(usize),
}

impl From<std::io::Error> for JournalError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<serde_json::Error> for JournalError {
    fn from(error: serde_json::Error) -> Self {
        Self::Corrupt(error.to_string())
    }
}

/// A journal handle may own live OS locks and is deliberately not `Clone`.
#[derive(Debug)]
pub struct LocalJournal {
    instance_id: InstanceId,
    dir: PathBuf,
    active_locks: HashMap<CommandId, File>,
}

impl LocalJournal {
    pub fn open(instance_id: InstanceId, base: &Path) -> Result<Self, JournalError> {
        let dir = base.join(&instance_id.0).join("operations");
        fs::create_dir_all(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Self {
            instance_id,
            dir,
            active_locks: HashMap::new(),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn append_prepared(&mut self, record: &JournalRecord) -> Result<(), JournalError> {
        let command_id = record.command_id.clone();
        if record.instance_id != self.instance_id {
            return Err(JournalError::Corrupt(format!(
                "prepared record instance {} does not match journal instance {}",
                record.instance_id.0, self.instance_id.0
            )));
        }
        if record.operation_id.0.trim().is_empty() {
            return Err(JournalError::Corrupt(
                "prepared record operation id must not be empty".to_owned(),
            ));
        }
        self.acquire_lock(&command_id)?;

        let path = self.command_file(&command_id)?;
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                self.release_lock(&command_id);
                return Err(JournalError::AlreadyExists(command_id));
            }
            Err(error) => {
                self.release_lock(&command_id);
                return Err(JournalError::Io(error.to_string()));
            }
        };

        let record = finalize_digest(JournalRecord {
            state: JournalState::Prepared,
            instance_id: self.instance_id.clone(),
            command_id,
            operation_id: record.operation_id.clone(),
            idempotency_key: record.idempotency_key.clone(),
            request_digest: record.request_digest.clone(),
            sequence: 1,
            previous_digest: ZERO_HASH_HEX.to_owned(),
            record_digest: String::new(),
        });
        if let Err(error) = self.write_record(&mut file, &record) {
            let command_id = record.command_id.clone();
            let _ = fs::remove_file(&path);
            self.release_lock(&command_id);
            return Err(error);
        }
        Ok(())
    }

    pub fn dispatch(&mut self, command_id: &CommandId) -> Result<(), JournalError> {
        self.append_state(command_id, JournalState::Dispatching, None)
    }

    pub fn observe(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), JournalError> {
        self.append_state(command_id, JournalState::Observed, Some(result))
    }

    pub fn terminal(&mut self, command_id: &CommandId) -> Result<(), JournalError> {
        self.append_state(command_id, JournalState::Terminal, None)
    }

    pub fn abandon(&mut self, command_id: &CommandId) -> Result<(), JournalError> {
        self.append_state(command_id, JournalState::Abandoned, None)
    }

    /// Acquire the command after a prior process has exited. Advisory locks are
    /// released automatically by the OS on crash; no PID liveness guess or
    /// stale lock-file deletion is involved.
    pub fn takeover(&mut self, command_id: &CommandId) -> Result<(), JournalError> {
        let path = self.command_file(command_id)?;
        if !path.is_file() {
            return Err(JournalError::NotFound(command_id.clone()));
        }
        self.acquire_lock(command_id)?;
        if let Err(error) = self.read_file(&path) {
            self.release_lock(command_id);
            return Err(error);
        }
        Ok(())
    }

    pub fn scan(&self) -> Result<Vec<JournalRecord>, JournalError> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.ends_with(".jsonl") {
                files.push(entry.path());
            }
        }
        files.sort();

        let mut out = Vec::new();
        for path in files {
            let remaining = MAX_SCAN_RECORDS.saturating_sub(out.len());
            if remaining == 0 {
                return Err(JournalError::CapacityBoundExceeded(out.len() + 1));
            }
            out.extend(self.read_file_bounded(&path, remaining)?);
        }
        Ok(out)
    }

    pub fn lookup(
        &self,
        command_id: &CommandId,
    ) -> Result<Option<Vec<JournalRecord>>, JournalError> {
        let path = self.command_file(command_id)?;
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(self.read_file(&path)?))
    }

    /// Capacity-prune oldest eligible `Prepared` commands. Pruning acquires
    /// the same exclusive OS lock used by writers and revalidates state+age
    /// while ownership is held before deleting the command file.
    pub fn prune_prepared(&mut self, policy: &PrunePolicy) -> Result<usize, JournalError> {
        let now = SystemTime::now();
        let mut prepared_count = 0usize;
        let mut candidates: Vec<(u64, CommandId, PathBuf)> = Vec::new();

        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            if !name.ends_with(".jsonl") {
                continue;
            }

            let command_id = CommandId(name.trim_end_matches(".jsonl").to_owned());
            let records = match self.read_file(&entry.path()) {
                Ok(records) => records,
                Err(_) => continue,
            };
            let Some(last) = records.last() else {
                continue;
            };
            if last.state != JournalState::Prepared {
                continue;
            }
            prepared_count = prepared_count.saturating_add(1);

            if self.is_locked(&command_id) {
                continue;
            }
            let modified = match fs::metadata(entry.path()).and_then(|metadata| metadata.modified()) {
                Ok(modified) => modified,
                Err(_) => continue,
            };
            let age = now
                .duration_since(modified)
                .map_or(0, |duration| duration.as_secs());
            if age >= policy.min_retention_seconds {
                candidates.push((age, command_id, entry.path()));
            }
        }

        let mut remaining_excess = prepared_count.saturating_sub(policy.max_prepared_count);
        if remaining_excess == 0 {
            return Ok(0);
        }

        candidates.sort_by_key(|(age, _, _)| Reverse(*age));
        let mut pruned = 0usize;
        for (_, command_id, path) in candidates {
            if remaining_excess == 0 {
                break;
            }
            match self.acquire_lock(&command_id) {
                Ok(()) => {}
                Err(JournalError::LockBusy(_)) => continue,
                Err(error) => return Err(error),
            }

            if !self.prepared_path_is_old_enough(&path, policy.min_retention_seconds) {
                self.release_lock(&command_id);
                continue;
            }

            match fs::remove_file(&path) {
                Ok(()) => {
                    self.release_lock(&command_id);
                    self.fsync_dir()?;
                    remaining_excess -= 1;
                    pruned += 1;
                }
                Err(error) => {
                    self.release_lock(&command_id);
                    return Err(JournalError::Io(error.to_string()));
                }
            }
        }
        Ok(pruned)
    }

    pub fn is_prepared_eligible_for_prune(
        &self,
        command_id: &CommandId,
        min_retention_seconds: u64,
    ) -> bool {
        let Ok(path) = self.command_file(command_id) else {
            return false;
        };
        !self.is_locked(command_id)
            && self.prepared_path_is_old_enough(&path, min_retention_seconds)
    }

    fn prepared_path_is_old_enough(&self, path: &Path, min_retention_seconds: u64) -> bool {
        let Ok(records) = self.read_file(path) else {
            return false;
        };
        let Some(last) = records.last() else {
            return false;
        };
        if last.state != JournalState::Prepared {
            return false;
        }
        let Ok(modified) = fs::metadata(path).and_then(|metadata| metadata.modified()) else {
            return false;
        };
        SystemTime::now()
            .duration_since(modified)
            .is_ok_and(|duration| duration.as_secs() >= min_retention_seconds)
    }

    fn command_file(&self, command_id: &CommandId) -> Result<PathBuf, JournalError> {
        validate_command_id(command_id)?;
        Ok(self.dir.join(format!("{}.jsonl", command_id.0)))
    }

    fn lock_path(&self, command_id: &CommandId) -> Result<PathBuf, JournalError> {
        let _ = self.command_file(command_id)?;
        Ok(self.dir.join(format!(".{}.lock", command_id.0)))
    }

    fn is_locked(&self, command_id: &CommandId) -> bool {
        if self.active_locks.contains_key(command_id) {
            return true;
        }
        let Some(lock_path) = self.lock_path(command_id).ok() else {
            return true;
        };
        let file = match OpenOptions::new().read(true).write(true).open(lock_path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => return false,
            Err(_) => return true,
        };
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => {
                let _ = FileExt::unlock(&file);
                false
            }
            Err(_) => true,
        }
    }

    fn acquire_lock(&mut self, command_id: &CommandId) -> Result<(), JournalError> {
        if self.active_locks.contains_key(command_id) {
            return Ok(());
        }
        let lock_path = self.lock_path(command_id)?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                return Err(JournalError::LockBusy(command_id.clone()));
            }
            Err(error) => return Err(JournalError::Io(error.to_string())),
        }

        // Diagnostic only; the file content has no ownership authority.
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(std::process::id().to_string().as_bytes())?;
        file.sync_all()?;
        self.fsync_dir()?;
        self.active_locks.insert(command_id.clone(), file);
        Ok(())
    }

    fn release_lock(&mut self, command_id: &CommandId) {
        if let Some(file) = self.active_locks.remove(command_id) {
            let _ = FileExt::unlock(&file);
        }
    }

    fn read_file(&self, path: &Path) -> Result<Vec<JournalRecord>, JournalError> {
        self.read_file_bounded(path, MAX_SCAN_RECORDS)
    }

    fn read_file_bounded(
        &self,
        path: &Path,
        max_records: usize,
    ) -> Result<Vec<JournalRecord>, JournalError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut records = Vec::new();
        let mut raw = Vec::new();
        loop {
            raw.clear();
            let read = reader.read_until(b'\n', &mut raw)?;
            if read == 0 {
                break;
            }
            if raw.last().copied() != Some(b'\n') {
                // Crash during append: only the final unterminated record may
                // be ignored. It is truncated under writer ownership before a
                // later append.
                break;
            }
            while raw
                .last()
                .is_some_and(|byte| *byte == b'\n' || *byte == b'\r')
            {
                raw.pop();
            }
            if raw.iter().all(|byte| byte.is_ascii_whitespace()) {
                continue;
            }
            if records.len() >= max_records {
                return Err(JournalError::CapacityBoundExceeded(records.len() + 1));
            }
            records.push(serde_json::from_slice(&raw)?);
        }
        validate_chain(&records)?;
        Ok(records)
    }

    fn truncate_incomplete_tail(&self, path: &Path) -> Result<(), JournalError> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let file_len = file.metadata()?.len();
        if file_len == 0 {
            return Ok(());
        }

        let mut reader = BufReader::new(file.try_clone()?);
        let mut raw = Vec::new();
        let mut complete_len = 0u64;
        loop {
            raw.clear();
            let read = reader.read_until(b'\n', &mut raw)?;
            if read == 0 || raw.last().copied() != Some(b'\n') {
                break;
            }
            let read = u64::try_from(read)
                .map_err(|_| JournalError::Corrupt("journal file length overflow".to_owned()))?;
            complete_len = complete_len
                .checked_add(read)
                .ok_or_else(|| JournalError::Corrupt("journal file length overflow".to_owned()))?;
        }

        if complete_len < file_len {
            file.set_len(complete_len)?;
            file.sync_all()?;
            self.fsync_dir()?;
        }
        Ok(())
    }

    fn append_state(
        &mut self,
        command_id: &CommandId,
        state: JournalState,
        result: Option<&OperationResult>,
    ) -> Result<(), JournalError> {
        if !self.active_locks.contains_key(command_id) {
            return Err(JournalError::NotOwner(command_id.clone()));
        }

        let path = self.command_file(command_id)?;
        self.truncate_incomplete_tail(&path)?;
        let records = self.read_file(&path)?;
        let Some(last) = records.last() else {
            return Err(JournalError::NotFound(command_id.clone()));
        };
        if !valid_transition(last.state, state) {
            return Err(JournalError::InvalidTransition {
                command_id: command_id.clone(),
                from: last.state,
                to: state,
            });
        }
        if let Some(result) = result {
            if result.command_id != *command_id
                || result.operation_id != last.operation_id
                || result.instance_id != self.instance_id
                || result.receipt.operation_id != last.operation_id.0
                || result.receipt.resolved_binding_digest != last.request_digest.0
            {
                return Err(JournalError::Corrupt(format!(
                    "observed result identity does not match command {}",
                    command_id.0
                )));
            }
        }

        let sequence = last.sequence.checked_add(1).ok_or_else(|| {
            JournalError::Corrupt(format!("sequence exhausted for command {}", command_id.0))
        })?;
        let record = finalize_digest(JournalRecord {
            state,
            instance_id: self.instance_id.clone(),
            command_id: command_id.clone(),
            operation_id: last.operation_id.clone(),
            idempotency_key: last.idempotency_key.clone(),
            request_digest: last.request_digest.clone(),
            sequence,
            previous_digest: last.record_digest.clone(),
            record_digest: String::new(),
        });
        let mut file = OpenOptions::new().append(true).open(&path)?;
        self.write_record(&mut file, &record)
    }

    fn write_record(&self, file: &mut File, record: &JournalRecord) -> Result<(), JournalError> {
        let mut json = serde_json::to_string(record)?;
        json.push('\n');
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        self.fsync_dir()
    }

    fn fsync_dir(&self) -> Result<(), JournalError> {
        File::open(&self.dir)?.sync_all()?;
        Ok(())
    }
}

impl Drop for LocalJournal {
    fn drop(&mut self) {
        for (_, file) in self.active_locks.drain() {
            let _ = FileExt::unlock(&file);
        }
    }
}

fn valid_transition(from: JournalState, to: JournalState) -> bool {
    matches!(
        (from, to),
        (JournalState::Prepared, JournalState::Dispatching)
            | (JournalState::Prepared, JournalState::Abandoned)
            | (JournalState::Dispatching, JournalState::Observed)
            | (JournalState::Observed, JournalState::Terminal)
    )
}

fn validate_command_id(command_id: &CommandId) -> Result<(), JournalError> {
    let name = command_id.0.as_str();
    let valid = !name.is_empty()
        && !name.starts_with('.')
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != "..";
    if valid {
        Ok(())
    } else {
        Err(JournalError::InvalidCommandId(command_id.clone()))
    }
}

fn canonical_payload(record: &JournalRecord) -> Vec<u8> {
    let mut bytes = Vec::new();
    let push = |bytes: &mut Vec<u8>, data: &[u8]| {
        bytes.extend_from_slice(&(data.len() as u64).to_be_bytes());
        bytes.extend_from_slice(data);
    };
    push(&mut bytes, state_str(record.state).as_bytes());
    push(&mut bytes, record.instance_id.0.as_bytes());
    push(&mut bytes, record.command_id.0.as_bytes());
    push(&mut bytes, record.operation_id.0.as_bytes());
    push(
        &mut bytes,
        record.idempotency_key.principal_ref.0.as_bytes(),
    );
    push(&mut bytes, record.idempotency_key.key_digest.as_bytes());
    push(&mut bytes, &record.idempotency_key.expires_at.to_be_bytes());
    push(&mut bytes, record.request_digest.0.as_bytes());
    push(&mut bytes, &record.sequence.to_be_bytes());
    push(&mut bytes, record.previous_digest.as_bytes());
    bytes
}

fn finalize_digest(mut record: JournalRecord) -> JournalRecord {
    record.record_digest = sha256_hex(&canonical_payload(&record));
    record
}

fn state_str(state: JournalState) -> &'static str {
    match state {
        JournalState::Prepared => "prepared",
        JournalState::Dispatching => "dispatching",
        JournalState::Observed => "observed",
        JournalState::Terminal => "terminal",
        JournalState::Abandoned => "abandoned",
    }
}

fn validate_chain(records: &[JournalRecord]) -> Result<(), JournalError> {
    let mut previous_digest: Option<&str> = None;
    let mut previous_state: Option<JournalState> = None;
    let mut expected_sequence = 1i64;
    let mut identity: Option<(
        &InstanceId,
        &CommandId,
        &OperationId,
        &IdempotencyKey,
        &RequestDigest,
    )> = None;

    for record in records {
        if record.sequence != expected_sequence {
            return Err(JournalError::ChainIntegrity(format!(
                "command {} sequence expected {expected_sequence}, found {}",
                record.command_id.0, record.sequence
            )));
        }
        expected_sequence = expected_sequence.checked_add(1).ok_or_else(|| {
            JournalError::ChainIntegrity("journal sequence exhausted".to_owned())
        })?;

        if let Some((instance, command, operation, key, digest)) = identity {
            if record.instance_id != *instance
                || record.command_id != *command
                || record.operation_id != *operation
                || record.idempotency_key != *key
                || record.request_digest != *digest
            {
                return Err(JournalError::ChainIntegrity(
                    "journal identity changed within a command chain".to_owned(),
                ));
            }
        } else {
            if record.operation_id.0.trim().is_empty() {
                return Err(JournalError::ChainIntegrity(
                    "journal operation id is missing; legacy record requires migration"
                        .to_owned(),
                ));
            }
            identity = Some((
                &record.instance_id,
                &record.command_id,
                &record.operation_id,
                &record.idempotency_key,
                &record.request_digest,
            ));
            if record.state != JournalState::Prepared {
                return Err(JournalError::ChainIntegrity(
                    "journal chain must begin in Prepared".to_owned(),
                ));
            }
        }

        if let Some(from) = previous_state {
            if !valid_transition(from, record.state) {
                return Err(JournalError::ChainIntegrity(format!(
                    "invalid journal transition {from:?} -> {:?}",
                    record.state
                )));
            }
        }

        let expected_previous = previous_digest.unwrap_or(ZERO_HASH_HEX);
        if record.previous_digest != expected_previous {
            return Err(JournalError::ChainIntegrity(format!(
                "record {} expects previous digest {expected_previous}, found {}",
                record.sequence, record.previous_digest
            )));
        }
        if record.record_digest != sha256_hex(&canonical_payload(record)) {
            return Err(JournalError::ChainIntegrity(format!(
                "record {} digest mismatch for command {}",
                record.sequence, record.command_id.0
            )));
        }
        previous_digest = Some(&record.record_digest);
        previous_state = Some(record.state);
    }
    Ok(())
}
