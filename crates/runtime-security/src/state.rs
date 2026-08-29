//! Durable canonical runtime-security state and idempotent cross-owner deltas.
//!
//! Application owns domain rows. Runtime-security owns Principal, Approval and
//! Authority state. A control-plane coordinator may persist a `SecurityDelta`
//! before the Application commit and replay it iff that Application operation
//! is durably bound, giving crash-safe cross-owner publication without copying
//! either owner's canonical state into the other.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use dxbot_core::types::{ApprovalId, OperationId, PrincipalRef};
use serde::{Deserialize, Serialize};

use crate::{
    ApprovalDecision, ApprovalManager, ApprovalState, AuthorityManager, Error,
    MembershipAuthorityBinding, ParkedGateBinding, ParkingManager, PrincipalManager,
};

const SECURITY_SCHEMA_VERSION: u32 = 1;
const MAX_SECURITY_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityAuditIntent {
    pub operation_id: OperationId,
    pub principal_ref: PrincipalRef,
    pub action: String,
    pub target: String,
    pub created_at: i64,
}

impl SecurityAuditIntent {
    pub fn new(
        operation_id: OperationId,
        principal_ref: PrincipalRef,
        action: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            operation_id,
            principal_ref,
            action: action.into(),
            target: target.into(),
            created_at: now_secs(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalWakeup {
    pub approval_id: ApprovalId,
    pub operation_id: OperationId,
    pub policy_generation: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MembershipBindingDelta {
    Upsert {
        binding: MembershipAuthorityBinding,
    },
    Revoke {
        binding_id: String,
        expected_generation: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecisionDelta {
    pub approval_id: ApprovalId,
    pub expected_revision: i64,
    pub decision: ApprovalDecision,
    pub by: PrincipalRef,
}

/// Durable parked-operation gate transition carried alongside a security delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ParkingDelta {
    /// Park a high-risk operation behind a bound approval before mutation.
    Park { binding: ParkedGateBinding },
    /// Record that the parked operation committed exactly once after approval.
    Continue { approval_id: ApprovalId },
    /// Record a terminal denial with durable reason audit semantics.
    Deny {
        approval_id: ApprovalId,
        reason: Option<String>,
    },
    /// Record that approval succeeded but original-request re-evaluation failed.
    Fail {
        approval_id: ApprovalId,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityDelta {
    pub membership: Option<MembershipBindingDelta>,
    pub approval: Option<ApprovalDecisionDelta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parking: Option<ParkingDelta>,
    pub audit_intent: Option<SecurityAuditIntent>,
}

impl SecurityDelta {
    pub fn audit_only(intent: SecurityAuditIntent) -> Self {
        Self {
            membership: None,
            approval: None,
            parking: None,
            audit_intent: Some(intent),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.membership.is_none()
            && self.approval.is_none()
            && self.parking.is_none()
            && self.audit_intent.is_none()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityState {
    pub principals: PrincipalManager,
    pub approvals: ApprovalManager,
    pub authority: AuthorityManager,
    #[serde(default)]
    pub parking: ParkingManager,
    audit_intents: BTreeMap<String, SecurityAuditIntent>,
    approval_wakeups: BTreeMap<String, ApprovalWakeup>,
}

impl SecurityState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_delta(&mut self, delta: &SecurityDelta) -> Result<(), Error> {
        if let Some(membership) = &delta.membership {
            match membership {
                MembershipBindingDelta::Upsert { binding } if binding.active => {
                    self.authority.apply_membership_binding(binding.clone())?;
                }
                MembershipBindingDelta::Upsert { binding } => self
                    .authority
                    .revoke_membership_binding(&binding.binding_id, binding.generation)?,
                MembershipBindingDelta::Revoke {
                    binding_id,
                    expected_generation,
                } => self
                    .authority
                    .revoke_membership_binding(binding_id, *expected_generation)?,
            }
        }

        if let Some(approval) = &delta.approval {
            let record = self.approvals.apply_decision_idempotently(
                &approval.approval_id,
                approval.expected_revision,
                approval.decision,
                &approval.by,
            )?;
            if record.state == ApprovalState::Approved {
                self.approval_wakeups
                    .entry(record.id.0.clone())
                    .or_insert_with(|| ApprovalWakeup {
                        approval_id: record.id,
                        operation_id: record.operation_id,
                        policy_generation: record.binding.policy_generation,
                        created_at: now_secs(),
                    });
            }
        }

        if let Some(parking) = &delta.parking {
            match parking {
                ParkingDelta::Park { binding } => {
                    self.parking.park(binding.clone(), now_secs())?;
                }
                ParkingDelta::Continue { approval_id } => {
                    self.parking.mark_continued(approval_id)?;
                    // The parked operation reached a terminal Application commit;
                    // its approval wakeup is fully consumed.
                    self.approval_wakeups.remove(&approval_id.0);
                }
                ParkingDelta::Deny {
                    approval_id,
                    reason,
                } => {
                    self.parking.mark_denied(approval_id, reason.clone())?;
                    // A denied gate never continues; drop any wakeup so no
                    // continuation can ever be driven from it.
                    self.approval_wakeups.remove(&approval_id.0);
                }
                ParkingDelta::Fail {
                    approval_id,
                    reason,
                } => {
                    self.parking.mark_failed(approval_id, reason.clone())?;
                    // Re-evaluation reached a terminal failure (for example
                    // stale CAS), so the wakeup is acknowledged without mutation.
                    self.approval_wakeups.remove(&approval_id.0);
                }
            }
        }

        if let Some(intent) = &delta.audit_intent {
            let key = audit_intent_key(intent);
            match self.audit_intents.get(&key) {
                Some(existing) if existing == intent => {}
                Some(_) => return Err(Error::AuditIntentConflict(key)),
                None => {
                    self.audit_intents.insert(key, intent.clone());
                }
            }
        }
        Ok(())
    }

    pub fn audit_intents(&self) -> Vec<SecurityAuditIntent> {
        self.audit_intents.values().cloned().collect()
    }

    pub fn approval_wakeups(&self) -> Vec<ApprovalWakeup> {
        self.approval_wakeups.values().cloned().collect()
    }

    pub fn acknowledge_approval_wakeup(&mut self, approval_id: &ApprovalId) {
        self.approval_wakeups.remove(&approval_id.0);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecuritySnapshot {
    schema_version: u32,
    state: SecurityState,
}

#[derive(Debug, Clone)]
pub struct SecurityStateStore {
    path: PathBuf,
}

impl SecurityStateStore {
    pub fn open(path: PathBuf) -> Result<(Self, SecurityState), io::Error> {
        match fs::symlink_metadata(&path) {
            Ok(metadata)
                if metadata.file_type().is_symlink() || !metadata.file_type().is_file() =>
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "security state is not a direct regular file: {}",
                        path.display()
                    ),
                ));
            }
            Ok(metadata) => {
                if metadata.len() > MAX_SECURITY_SNAPSHOT_BYTES as u64 {
                    return Err(io::Error::new(
                        io::ErrorKind::OutOfMemory,
                        "security state exceeds 16 MiB capacity",
                    ));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let store = Self { path };
        let state = match fs::read(&store.path) {
            Ok(bytes) => {
                let snapshot: SecuritySnapshot =
                    serde_json::from_slice(&bytes).map_err(|error| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("cannot decode security state: {error}"),
                        )
                    })?;
                if snapshot.schema_version != SECURITY_SCHEMA_VERSION {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "unsupported security state schema {}",
                            snapshot.schema_version
                        ),
                    ));
                }
                snapshot.state
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => SecurityState::new(),
            Err(error) => return Err(error),
        };
        Ok((store, state))
    }

    pub fn persist(&self, state: &SecurityState) -> Result<(), io::Error> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let snapshot = SecuritySnapshot {
            schema_version: SECURITY_SCHEMA_VERSION,
            state: state.clone(),
        };
        let bytes = serde_json::to_vec(&snapshot)
            .map_err(|error| io::Error::other(format!("cannot encode security state: {error}")))?;
        if bytes.len() > MAX_SECURITY_SNAPSHOT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                "security state exceeds 16 MiB capacity",
            ));
        }
        let temp = temp_path(&self.path);
        let write_result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
            }
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, &self.path)?;
            File::open(parent)?.sync_all()?;
            Ok::<(), io::Error>(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        write_result
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn audit_intent_key(intent: &SecurityAuditIntent) -> String {
    length_prefixed(&[
        intent.operation_id.0.as_str(),
        intent.action.as_str(),
        intent.target.as_str(),
    ])
}

fn length_prefixed(parts: &[&str]) -> String {
    let mut key = String::new();
    for part in parts {
        key.push_str(&part.len().to_string());
        key.push(':');
        key.push_str(part);
    }
    key
}

fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("security-state.json");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), nonce))
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use dxbot_core::types::{BotId, BotSelector, ProjectId, ProjectSelector, ScopeSelector};

    use crate::{ApprovalBinding, ApprovalDecision};

    use super::*;

    fn membership_binding(generation: i64, active: bool) -> MembershipAuthorityBinding {
        MembershipAuthorityBinding {
            binding_id: "member-a".to_owned(),
            scope: ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
                "project-a".to_owned(),
            ))),
            member_bot: BotSelector::CanonicalId(BotId("bot-a".to_owned())),
            role: "member".to_owned(),
            generation,
            active,
        }
    }

    #[test]
    fn membership_delta_is_idempotent_and_does_not_create_principal() {
        let mut state = SecurityState::new();
        let binding = membership_binding(1, true);
        let delta = SecurityDelta {
            membership: Some(MembershipBindingDelta::Upsert {
                binding: binding.clone(),
            }),
            approval: None,
            parking: None,
            audit_intent: None,
        };
        state.apply_delta(&delta).expect("first apply");
        state.apply_delta(&delta).expect("replay apply");
        assert_eq!(
            state.authority.membership_binding("member-a"),
            Some(binding)
        );
        assert!(
            state
                .principals
                .resolve_principal(&PrincipalRef("bot:bot-a".to_owned()))
                .is_err()
        );
    }

    #[test]
    fn inactive_upsert_is_generation_advancing_idempotent_revoke() {
        let mut state = SecurityState::new();
        state
            .apply_delta(&SecurityDelta {
                membership: Some(MembershipBindingDelta::Upsert {
                    binding: membership_binding(1, true),
                }),
                approval: None,
                parking: None,
                audit_intent: None,
            })
            .expect("create binding");
        let revoke = SecurityDelta {
            membership: Some(MembershipBindingDelta::Upsert {
                binding: membership_binding(1, false),
            }),
            approval: None,
            parking: None,
            audit_intent: None,
        };
        state.apply_delta(&revoke).expect("revoke");
        state.apply_delta(&revoke).expect("replay revoke");
        let tombstone = state
            .authority
            .membership_binding("member-a")
            .expect("binding");
        assert!(!tombstone.active);
        assert_eq!(tombstone.generation, 2);
    }

    #[test]
    fn audit_intent_keys_are_unambiguous() {
        let first = SecurityAuditIntent {
            operation_id: OperationId("a:b".to_owned()),
            principal_ref: PrincipalRef("p".to_owned()),
            action: "c".to_owned(),
            target: "d".to_owned(),
            created_at: 1,
        };
        let second = SecurityAuditIntent {
            operation_id: OperationId("a".to_owned()),
            principal_ref: PrincipalRef("p".to_owned()),
            action: "b:c".to_owned(),
            target: "d".to_owned(),
            created_at: 1,
        };
        assert_ne!(audit_intent_key(&first), audit_intent_key(&second));
    }

    #[test]
    fn approval_decision_wakeup_and_audit_survive_restart() {
        let root = std::env::temp_dir().join(format!(
            "dxbot-security-state-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        let path = root.join("security-state.json");
        let (store, mut state) = SecurityStateStore::open(path.clone()).expect("open state");
        let approver = PrincipalRef("local:i:uid:1000".to_owned());
        let operation_id = OperationId("operation-a".to_owned());
        let approval_id = state
            .approvals
            .create_bound_approval(
                operation_id.clone(),
                ApprovalBinding {
                    action: "delete".to_owned(),
                    target: "project:alpha".to_owned(),
                    policy_generation: 7,
                },
                vec![approver.clone()],
            )
            .expect("approval");
        let delta = SecurityDelta {
            membership: None,
            approval: Some(ApprovalDecisionDelta {
                approval_id: approval_id.clone(),
                expected_revision: 1,
                decision: ApprovalDecision::Approve,
                by: approver.clone(),
            }),
            parking: None,
            audit_intent: Some(SecurityAuditIntent {
                operation_id: OperationId("decision-operation".to_owned()),
                principal_ref: approver,
                action: "approval-approve".to_owned(),
                target: format!("approval:{}", approval_id.0),
                created_at: 1,
            }),
        };
        state.apply_delta(&delta).expect("apply");
        state.apply_delta(&delta).expect("idempotent replay");
        store.persist(&state).expect("persist");

        let (_reopened_store, reopened) =
            SecurityStateStore::open(path).expect("reopen persisted state");
        assert_eq!(
            reopened
                .approvals
                .get_approval(&approval_id)
                .expect("approval restored")
                .state,
            ApprovalState::Approved
        );
        assert_eq!(reopened.approval_wakeups().len(), 1);
        assert_eq!(reopened.approval_wakeups()[0].operation_id, operation_id);
        assert_eq!(reopened.approval_wakeups()[0].policy_generation, 7);
        assert_eq!(reopened.audit_intents().len(), 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn security_snapshot_capacity_failure_preserves_prior_durable_state() {
        let root = std::env::temp_dir().join(format!(
            "dxbot-security-pressure-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp root");
        let path = root.join("security-state.json");
        let (store, state) = SecurityStateStore::open(path.clone()).expect("open");
        store.persist(&state).expect("baseline");
        let baseline = fs::read(&path).expect("baseline bytes");

        let mut oversized = state;
        oversized.audit_intents.insert(
            "oversized".to_owned(),
            SecurityAuditIntent {
                operation_id: OperationId("operation-pressure".to_owned()),
                principal_ref: PrincipalRef("principal-pressure".to_owned()),
                action: "x".repeat(MAX_SECURITY_SNAPSHOT_BYTES + 1),
                target: "runtime".to_owned(),
                created_at: 1,
            },
        );
        let error = store.persist(&oversized).expect_err("capacity failure");
        assert_eq!(error.kind(), io::ErrorKind::OutOfMemory);
        assert_eq!(fs::read(&path).expect("durable state"), baseline);
        let (_, restored) = SecurityStateStore::open(path).expect("baseline restores");
        assert!(restored.audit_intents().is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
