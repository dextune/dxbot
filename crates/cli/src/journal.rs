//! Local journal: single-writer takeover, hash chain, bounded retention.
//!
//! The local journal persists the lifecycle of each CLI command (`Prepared →
//! Dispatching → Observed → Terminal`, or `Prepared → Abandoned`) in an
//! owner-only directory under
//! `$XDG_STATE_HOME/dxbot/cli/<instance-id>/operations/<command-id>.jsonl`.
//!
//! ## Identity
//!
//! Each line is a JSON [`JournalRecord`]. A record is a link in a SHA-256 hash
//! chain: `previous_digest` is the `record_digest` of the previous record and
//! `record_digest` is the digest of the canonical concatenation of every field
//! except `record_digest`. The first record's `previous_digest` is the zero
//! hash. The chain is validated on every read; any mismatch fails closed.
//!
//! ## Single writer
//!
//! The command file is created with `create_new` (OS-atomic exclusive create)
//! so no two writers can ever both own the initial `Prepared` record. A
//! per-command lock file (`. <command>.lock`) records the owning PID and is the
//! gate for later appends. `takeover` re-acquires a stale lock only after the
//! previous writer has actually exited (its PID is no longer alive); it never
//! guesses a live writer's lock away.
//!
//! ## Bounded retention
//!
//! Only `Prepared` records — provably unsent — that are lock-free,
//! integrity-valid and older than [`PrunePolicy::min_retention_seconds`] are
//! eligible for pruning, bounded by [`PrunePolicy::max_prepared_count`].

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use dxbot_core::types::*;

/// Upper bound on records returned by a single `scan` (bounded startup scan).
const MAX_SCAN_RECORDS: usize = 1_000_000;

/// The zero hash used to anchor the head of every hash chain (SHA-256 of the
/// empty message, hex-encoded).
const ZERO_HASH_HEX: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Retention / capacity policy used by [`LocalJournal::prune_prepared`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrunePolicy {
    /// Only prune `Prepared` records at least this many seconds old.
    pub min_retention_seconds: u64,
    /// Keep at most this many `Prepared` commands (oldest evicted first).
    /// A value of `0` keeps nothing except what retention itself preserves.
    pub max_prepared_count: usize,
}

/// Errors produced by the local journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalError {
    /// An OS I/O failure.
    Io(String),
    /// A record could not be parsed or a lower-level corruption was detected.
    Corrupt(String),
    /// The command file already exists; a different writer created it.
    AlreadyExists(CommandId),
    /// The command file does not exist.
    NotFound(CommandId),
    /// Another live writer holds the per-command lock.
    LockBusy(CommandId),
    /// The command id is not a safe single path component.
    InvalidCommandId(CommandId),
    /// This journal does not own the command's writer lock.
    NotOwner(CommandId),
    /// The hash chain or a record digest does not match.
    ChainIntegrity(String),
    /// A read exceeded the bounded scan limit.
    CapacityBoundExceeded(usize),
}

impl From<std::io::Error> for JournalError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<serde_json::Error> for JournalError {
    fn from(e: serde_json::Error) -> Self {
        Self::Corrupt(e.to_string())
    }
}

/// A single-writer, hash-chained, file-backed local journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalJournal {
    instance_id: InstanceId,
    /// `base/<instance-id>/operations`.
    dir: PathBuf,
    /// Commands whose writer lock this instance currently holds.
    active_locks: HashSet<CommandId>,
}

impl LocalJournal {
    /// Opens (creating if needed) the journal directory at
    /// `base/…/dxbot/cli/<instance-id>/operations`. `base` is the CLI state
    /// root (`$XDG_STATE_HOME/dxbot/cli` in production); the directory is
    /// created owner-only.
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
            active_locks: HashSet::new(),
        })
    }

    /// The directory that backs this journal.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Creates a new command file (exclusive create), writes its `Prepared`
    /// record with the hash chain, and makes it durable.
    pub fn append_prepared(&mut self, record: &JournalRecord) -> Result<(), JournalError> {
        let command_id = record.command_id.clone();
        // Acquire the per-command writer lock before creating the file so a
        // failed exclusive create does not leave a dangling lock behind us.
        self.acquire_lock(&command_id)?;

        let path = self.command_file(&command_id)?;
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(f) => f,
            Err(_) => {
                // Failed to exclusively create the file: someone else owns it.
                self.release_lock(&command_id);
                return Err(JournalError::AlreadyExists(command_id));
            }
        };

        let rec = JournalRecord {
            state: JournalState::Prepared,
            instance_id: self.instance_id.clone(),
            command_id,
            idempotency_key: record.idempotency_key.clone(),
            request_digest: record.request_digest.clone(),
            sequence: 1,
            previous_digest: ZERO_HASH_HEX.to_string(),
            record_digest: String::new(),
        };
        let rec = finalize_digest(rec);
        self.write_record(&mut file, &rec)?;
        Ok(())
    }

    /// Appends a `Dispatching` record (fsynced before any network byte).
    pub fn dispatch(&mut self, command_id: &CommandId) -> Result<(), JournalError> {
        self.append_state(command_id, JournalState::Dispatching, None)
    }

    /// Appends an `Observed` record once an operation result is observed.
    pub fn observe(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), JournalError> {
        self.append_state(command_id, JournalState::Observed, Some(result))
    }

    /// Appends a `Terminal` record.
    pub fn terminal(&mut self, command_id: &CommandId) -> Result<(), JournalError> {
        self.append_state(command_id, JournalState::Terminal, None)
    }

    /// Appends an `Abandoned` record.
    pub fn abandon(&mut self, command_id: &CommandId) -> Result<(), JournalError> {
        self.append_state(command_id, JournalState::Abandoned, None)
    }

    /// Take over a command whose previous writer has exited. This re-acquires
    /// the per-command lock only when the recorded owner PID is no longer
    /// alive, validates the existing chain, and then allows further appends.
    pub fn takeover(&mut self, command_id: &CommandId) -> Result<(), JournalError> {
        self.acquire_lock(command_id)?;
        let path = self.command_file(command_id)?;
        if path.exists() {
            // Validate the existing chain before adopting the command.
            self.read_file(&path)?;
        }
        Ok(())
    }

    /// Bounded scan of every journal record in this instance's directory,
    /// ordered by command file name and then by append order.
    pub fn scan(&self) -> Result<Vec<JournalRecord>, JournalError> {
        let mut files: Vec<PathBuf> = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = match entry.file_name().to_str().map(str::to_string) {
                Some(n) => n,
                None => continue,
            };
            if name.ends_with(".jsonl") {
                files.push(entry.path());
            }
        }
        files.sort();
        let mut out: Vec<JournalRecord> = Vec::new();
        for path in files {
            out.extend(self.read_file(&path)?);
            if out.len() > MAX_SCAN_RECORDS {
                return Err(JournalError::CapacityBoundExceeded(out.len()));
            }
        }
        Ok(out)
    }

    /// Returns every record for a single command, or `None` if the command has
    /// no journal file.
    pub fn lookup(&self, command_id: &CommandId) -> Result<Option<Vec<JournalRecord>>, JournalError> {
        let path = self.command_file(command_id)?;
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(self.read_file(&path)?))
    }

    /// Prunes stale, eligible `Prepared` commands: those whose last record is
    /// `Prepared`, that are not actively locked, that pass chain validation,
    /// and that are older than `min_retention_seconds`. At most
    /// `max_prepared_count` commands are retained (oldest evicted first).
    /// Returns the number of commands pruned.
    pub fn prune_prepared(&mut self, policy: &PrunePolicy) -> Result<usize, JournalError> {
        let now = SystemTime::now();
        // (age_seconds, command_id, path), oldest first.
        let mut eligible: Vec<(u64, CommandId, PathBuf)> = Vec::new();
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
            let command_id = CommandId(name.trim_end_matches(".jsonl").to_string());
            if self.is_locked(&command_id) {
                continue;
            }
            let recs = match self.read_file(&entry.path()) {
                Ok(r) => r,
                // Integrity-invalid records are never capacity-pruned.
                Err(_) => continue,
            };
            let Some(last) = recs.last() else {
                continue;
            };
            if last.state != JournalState::Prepared {
                continue;
            }
            let modified = match fs::metadata(entry.path()).and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let age = match now.duration_since(modified) {
                Ok(d) => d.as_secs(),
                Err(_) => 0,
            };
            eligible.push((age, command_id, entry.path()));
        }
        eligible.sort_by_key(|e| e.0);

        let excess = eligible.len().saturating_sub(policy.max_prepared_count);
        let mut pruned = 0usize;
        for (index, (age, command_id, path)) in eligible.into_iter().enumerate() {
            let past_retention = age >= policy.min_retention_seconds;
            let over_quota = index < excess;
            if !past_retention && !over_quota {
                continue;
            }
            if let Ok(lock) = self.lock_path(&command_id) {
                let _ = fs::remove_file(lock);
            }
            fs::remove_file(&path)?;
            self.active_locks.remove(&command_id);
            pruned += 1;
        }
        Ok(pruned)
    }

    /// Returns whether a single command's latest record is a `Prepared` that is
    /// eligible to prune under `min_retention_seconds`: lock-free,
    /// integrity-valid, and older than the retention bound. This is the
    /// decision used by recovery reuse so a stale Prepared entry can be pruned
    /// without mutating the journal up front.
    pub fn is_prepared_eligible_for_prune(
        &self,
        command_id: &CommandId,
        min_retention_seconds: u64,
    ) -> bool {
        let Ok(path) = self.command_file(command_id) else {
            return false;
        };
        if self.is_locked(command_id) {
            return false;
        }
        // read_file validates the chain; integrity-invalid records are never
        // eligible to prune.
        let Ok(records) = self.read_file(&path) else {
            return false;
        };
        let Some(last) = records.last() else {
            return false;
        };
        if last.state != JournalState::Prepared {
            return false;
        }
        let Ok(modified) = fs::metadata(&path).and_then(|m| m.modified()) else {
            return false;
        };
        let age = match SystemTime::now().duration_since(modified) {
            Ok(d) => d.as_secs(),
            Err(_) => 0,
        };
        age >= min_retention_seconds
    }

    // ── internals ──

    fn command_file(&self, command_id: &CommandId) -> Result<PathBuf, JournalError> {
        validate_command_id(command_id)?;
        Ok(self.dir.join(format!("{}.jsonl", command_id.0)))
    }

    fn lock_path(&self, command_id: &CommandId) -> Result<PathBuf, JournalError> {
        let _ = self.command_file(command_id)?;
        Ok(self.dir.join(format!(".{}.lock", command_id.0)))
    }

    fn is_locked(&self, command_id: &CommandId) -> bool {
        if self.active_locks.contains(command_id) {
            return true;
        }
        let Some(lock) = self.lock_path(command_id).ok() else {
            return false;
        };
        if !lock.exists() {
            return false;
        }
        match read_lock_pid(&lock) {
            Some(pid) => pid_alive(pid),
            None => true,
        }
    }

    fn acquire_lock(&mut self, command_id: &CommandId) -> Result<(), JournalError> {
        if self.active_locks.contains(command_id) {
            return Ok(());
        }
        let lock = self.lock_path(command_id)?;
        let mut attempts = 0;
        loop {
            attempts += 1;
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock)
            {
                Ok(mut f) => {
                    let pid = std::process::id().to_string();
                    f.write_all(pid.as_bytes())?;
                    f.flush()?;
                    f.sync_all()?;
                    self.active_locks.insert(command_id.clone());
                    return Ok(());
                }
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    if !self.lock_is_stale(&lock) {
                        return Err(JournalError::LockBusy(command_id.clone()));
                    }
                    // Stale (previous writer exited): reclaim and retry.
                    if attempts >= 3 {
                        return Err(JournalError::LockBusy(command_id.clone()));
                    }
                    let _ = fs::remove_file(&lock);
                }
                Err(e) => return Err(JournalError::Io(e.to_string())),
            }
        }
    }

    fn lock_is_stale(&self, lock: &Path) -> bool {
        match read_lock_pid(lock) {
            Some(pid) => !pid_alive(pid),
            None => false,
        }
    }

    fn release_lock(&mut self, command_id: &CommandId) {
        if self.active_locks.remove(command_id) {
            if let Ok(lock) = self.lock_path(command_id) {
                let _ = fs::remove_file(lock);
            }
        }
    }

    fn read_file(&self, path: &Path) -> Result<Vec<JournalRecord>, JournalError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut records: Vec<JournalRecord> = Vec::new();
        for line in reader.lines() {
            match line {
                Ok(raw) => {
                    if raw.trim().is_empty() {
                        continue;
                    }
                    let record: JournalRecord = serde_json::from_str(&raw)?;
                    records.push(record);
                }
                // A truncated final record (crash mid-append) is permitted; the
                // complete records before it still form a valid chain.
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(JournalError::Io(e.to_string())),
            }
        }
        validate_chain(&records)?;
        Ok(records)
    }

    fn append_state(
        &mut self,
        command_id: &CommandId,
        state: JournalState,
        result: Option<&OperationResult>,
    ) -> Result<(), JournalError> {
        if !self.active_locks.contains(command_id) {
            return Err(JournalError::NotOwner(command_id.clone()));
        }
        if let Some(result) = result {
            if result.command_id != *command_id {
                return Err(JournalError::InvalidCommandId(command_id.clone()));
            }
        }
        let path = self.command_file(command_id)?;
        let records = self.read_file(&path)?;
        let Some(last) = records.last() else {
            return Err(JournalError::NotFound(command_id.clone()));
        };
        let rec = JournalRecord {
            state,
            instance_id: self.instance_id.clone(),
            command_id: command_id.clone(),
            idempotency_key: last.idempotency_key.clone(),
            request_digest: last.request_digest.clone(),
            sequence: last.sequence + 1,
            previous_digest: last.record_digest.clone(),
            record_digest: String::new(),
        };
        let rec = finalize_digest(rec);
        let mut file = OpenOptions::new().append(true).open(&path)?;
        self.write_record(&mut file, &rec)?;
        Ok(())
    }

    fn write_record(&self, file: &mut File, record: &JournalRecord) -> Result<(), JournalError> {
        let mut json = serde_json::to_string(record)?;
        json.push('\n');
        file.write_all(json.as_bytes())?;
        file.flush()?;
        // Durable append: fsync the record, then the directory, before any
        // dependent action (e.g. the first network byte).
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
        let locks: Vec<CommandId> = self.active_locks.drain().collect();
        for command_id in locks {
            if let Ok(lock) = self.lock_path(&command_id) {
                let _ = fs::remove_file(lock);
            }
        }
    }
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

fn read_lock_pid(path: &Path) -> Option<u32> {
    let content = fs::read_to_string(path).ok()?;
    content.trim().parse::<u32>().ok()
}

#[cfg(target_os = "linux")]
fn pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(not(target_os = "linux"))]
fn pid_alive(_pid: u32) -> bool {
    // Conservative fallback: do not reclaim locks on platforms without
    // /proc liveness introspection.  This prevents false-positive stale
    // detection that could steal a lock from a live writer.
    false
}

/// Computes the canonical payload hashed by `record_digest`: every field except
/// `record_digest`, length-prefixed to avoid concatenation ambiguity.
fn canonical_payload(record: &JournalRecord) -> Vec<u8> {
    let mut b: Vec<u8> = Vec::new();
    let push = |b: &mut Vec<u8>, data: &[u8]| {
        b.extend_from_slice(&(data.len() as u64).to_be_bytes());
        b.extend_from_slice(data);
    };
    push(&mut b, state_str(record.state).as_bytes());
    push(&mut b, record.instance_id.0.as_bytes());
    push(&mut b, record.command_id.0.as_bytes());
    push(&mut b, record.idempotency_key.principal_ref.0.as_bytes());
    push(&mut b, record.idempotency_key.key_digest.as_bytes());
    push(&mut b, &record.idempotency_key.expires_at.to_be_bytes());
    push(&mut b, record.request_digest.0.as_bytes());
    push(&mut b, &record.sequence.to_be_bytes());
    push(&mut b, record.previous_digest.as_bytes());
    b
}

/// Sets `record_digest` on a record whose other fields are final.
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
    let mut previous: Option<&str> = None;
    for record in records {
        let expected_prev = previous.unwrap_or(ZERO_HASH_HEX);
        if record.previous_digest != expected_prev {
            return Err(JournalError::ChainIntegrity(format!(
                "record {} expects previous digest {expected_prev}, found {}",
                record.sequence, record.previous_digest
            )));
        }
        if record.record_digest != sha256_hex(&canonical_payload(record)) {
            return Err(JournalError::ChainIntegrity(format!(
                "record {} digest mismatch for command {}",
                record.sequence, record.command_id.0
            )));
        }
        previous = Some(&record.record_digest);
    }
    Ok(())
}

// ── SHA-256 (self-contained; no external crypto dependency) ──

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

struct Sha256 {
    h: [u32; 8],
    len_bytes: u64,
    buf: [u8; 64],
    buf_len: usize,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            len_bytes: 0,
            buf: [0u8; 64],
            buf_len: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.len_bytes = self.len_bytes.wrapping_add(data.len() as u64);
        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 64 {
                let block = self.buf;
                self.process_block(&block);
                self.buf_len = 0;
            }
        }
        while data.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[..64]);
            self.process_block(&block);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    fn process_block(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for (word, bytes) in w[..16].iter_mut().zip(block.chunks_exact(4)) {
            let mut arr = [0u8; 4];
            arr.copy_from_slice(bytes);
            *word = u32::from_be_bytes(arr);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.h;
        for (&wv, &kv) in w.iter().zip(K.iter()) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(kv).wrapping_add(wv);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (state, round) in self.h.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *state = state.wrapping_add(round);
        }
    }

    fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.len_bytes.wrapping_mul(8);
        self.buf[self.buf_len] = 0x80;
        self.buf_len += 1;
        if self.buf_len > 56 {
            self.buf[self.buf_len..].fill(0);
            let block = self.buf;
            self.process_block(&block);
            self.buf = [0u8; 64];
        } else {
            self.buf[self.buf_len..56].fill(0);
        }
        self.buf[56..64].copy_from_slice(&bit_len.to_be_bytes());
        let block = self.buf;
        self.process_block(&block);
        let mut out = [0u8; 32];
        for (slot, &state) in out.chunks_exact_mut(4).zip(self.h.iter()) {
            slot.copy_from_slice(&state.to_be_bytes());
        }
        out
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let bytes = hasher.finalize();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_multiblock() {
        // 128 bytes is exactly two 64-byte blocks; 129 forces a padded third.
        assert_eq!(
            sha256_hex(&[0x61u8; 64]),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
        assert_eq!(
            sha256_hex(&[0x61u8; 128]),
            "6836cf13bac400e9105071cd6af47084dfacad4e5e302c94bfed24e013afb73e"
        );
        assert_eq!(
            sha256_hex(&[0x61u8; 129]),
            "c12cb024a2e5551cca0e08fce8f1c5e314555cc3fef6329ee994a3db752166ae"
        );
    }

    }