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
    MembershipAuthorityBinding, PrincipalManager,
};

const SECURITY_SCHEMA_VERSION: u32 = 1;

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
    Upsert { binding: MembershipAuthorityBinding },
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityDelta {
    pub membership: Option<MembershipBindingDelta>,
    pub approval: Option<ApprovalDecisionDelta>,
    pub audit_intent: Option<SecurityAuditIntent>,
}

impl SecurityDelta {
    pub fn audit_only(intent: SecurityAuditIntent) -> Self {
        Self {
            membership: None,
            approval: None,
            audit_intent: Some(intent),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.membership.is_none() && self.approval.is_none() && self.audit_intent.is_none()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityState {
    pub principals: PrincipalManager,
    pub approvals: ApprovalManager,
    pub authority: AuthorityManager,
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
                MembershipBindingDelta::Upsert { binding } => {
                    self.authority.apply_membership_binding(binding.clone())?;
                }
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
        if let Ok(metadata) = fs::symlink_metadata(&path) {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("security state is not a direct regular file: {}", path.display()),
                ));
            }
        }
        let store = Self { path };
        let state = match fs::read(&store.path) {
            Ok(bytes) => {
                let snapshot: SecuritySnapshot = serde_json::from_slice(&bytes).map_err(|error| {
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
    format!(
        "{}:{}:{}",
        intent.operation_id.0, intent.action, intent.target
    )
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

    use super::*;

    #[test]
    fn membership_delta_is_idempotent_and_does_not_create_principal() {
        let mut state = SecurityState::new();
        let binding = MembershipAuthorityBinding {
            binding_id: "member-a".to_owned(),
            scope: ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
                "project-a".to_owned(),
            ))),
            member_bot: BotSelector::CanonicalId(BotId("bot-a".to_owned())),
            role: "member".to_owned(),
            generation: 1,
            active: true,
        };
        let delta = SecurityDelta {
            membership: Some(MembershipBindingDelta::Upsert { binding: binding.clone() }),
            approval: None,
            audit_intent: None,
        };
        state.apply_delta(&delta).expect("first apply");
        state.apply_delta(&delta).expect("replay apply");
        assert_eq!(state.authority.membership_binding("member-a"), Some(binding));
        assert!(state
            .principals
            .resolve_principal(&PrincipalRef("bot:bot-a".to_owned()))
            .is_err());
    }
}
