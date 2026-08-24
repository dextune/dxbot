use serde::{Deserialize, Serialize};

// ── Domain identity types ──

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstanceId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BotId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThreadId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperationId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApprovalId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalRef(pub String);

// ── Selectors ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum BotSelector {
    CanonicalId(BotId),
    ScopedExact(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ConversationSelector {
    ConversationId(ConversationId),
    BotMain(BotSelector),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ThreadSelector {
    CanonicalId(ThreadId),
    ScopedExact(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TaskSelector {
    CanonicalId(TaskId),
    ScopedExact(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProjectSelector {
    CanonicalId(ProjectId),
    VisibleExact(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ChannelSelector {
    CanonicalId(ChannelId),
    ProjectExact { project: String, name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OperationSelector {
    OperationId(OperationId),
    CommandIdKey { command_id: CommandId, key_digest: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ScopeSelector {
    Bot(BotSelector),
    Project(ProjectSelector),
    Channel(ChannelSelector),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ApprovalSelector {
    CanonicalId(ApprovalId),
    Operation(OperationSelector),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProviderSelector {
    CanonicalId(ProviderId),
    Exact(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ProcessSelector {
    CanonicalId(ProcessId),
    ScopedExact(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MemorySelector {
    CanonicalId(MemoryId),
    ScopedExact(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryScopeSelector {
    pub scope: ScopeSelector,
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SideEffectSelector {
    CanonicalId(String),
    Operation(OperationSelector),
}

// ── Content source ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ContentSource {
    Text { value: String },
    InputFile { path: String },
    Stdin,
    ArtifactRef { artifact_id: String, digest: String },
}

// ── Command payload ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandPayload {
    pub command_key: String,
    pub principal_ref: PrincipalRef,
    pub instance_id: InstanceId,
    pub canonical_target: CanonicalTarget,
    pub cas: Option<CasConditions>,
    pub content: Option<ContentSource>,
    pub semantic_options: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CanonicalTarget {
    Instance(InstanceId),
    Bot { id: BotId, revision: i64 },
    Conversation { id: ConversationId, revision: i64 },
    Thread { id: ThreadId, parent_id: Option<ConversationId>, revision: i64 },
    Task { id: TaskId, revision: i64, execution_generation: Option<i64> },
    Project { id: ProjectId, revision: i64 },
    Channel { id: ChannelId, project_id: ProjectId, revision: i64 },
    Operation { operation_id: OperationId, command_id: CommandId },
    Approval { id: ApprovalId, revision: i64 },
    Memory { id: MemoryId, revision: i64 },
    Provider { id: ProviderId, generation: i64 },
    Process { id: ProcessId },
    SideEffect { id: String, revision: i64 },
    Membership { scope: ScopeSelector, member_bot: BotSelector },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CasConditions {
    pub if_revision: Option<i64>,
    pub if_generation: Option<i64>,
    pub if_host_generation: Option<i64>,
    pub if_execution_generation: Option<i64>,
    pub if_source_revision: Option<i64>,
    pub if_scope_revision: Option<i64>,
    pub if_project_revision: Option<i64>,
    pub if_channel_revision: Option<i64>,
    pub if_membership_generation: Option<i64>,
    pub if_proposal_revision: Option<i64>,
    pub if_target_scope_revision: Option<i64>,
    pub if_receipt_revision: Option<i64>,
}

// ── Request / Response types ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestDigest(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdempotencyKey {
    pub principal_ref: PrincipalRef,
    pub key_digest: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationRequest {
    pub command_id: CommandId,
    pub idempotency_key: IdempotencyKey,
    pub request_digest: RequestDigest,
    pub new_operation_id: OperationId,
    pub payload: CommandPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationResult {
    pub operation_id: OperationId,
    pub command_id: CommandId,
    pub instance_id: InstanceId,
    pub receipt: super::receipt::ReceiptRecord,
    pub status: String,
    pub committed_payload: Option<serde_json::Value>,
    pub error: Option<super::error::DxbotError>,
    pub operation_may_continue: bool,
}

// ── Journal types ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JournalState {
    Prepared,
    Dispatching,
    Observed,
    Terminal,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalRecord {
    pub state: JournalState,
    pub instance_id: InstanceId,
    pub command_id: CommandId,
    /// Original OperationId allocated before the durable Prepared record.
    pub operation_id: OperationId,
    pub idempotency_key: IdempotencyKey,
    pub request_digest: RequestDigest,
    pub sequence: i64,
    pub previous_digest: String,
    pub record_digest: String,
}

// ── CLI rendering ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    Human,
    Json,
    Jsonl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

// ── Version compatibility ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    pub client_version: String,
    pub supported_protocol_versions: Vec<String>,
    pub supported_schema_versions: Vec<String>,
    pub remote_compatibility: Option<RemoteVersionInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteVersionInfo {
    pub runtime_version: String,
    pub protocol_version: String,
    pub schema_version: String,
    pub compatible: bool,
}

// ── CLI global options ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliGlobalOptions {
    pub profile: Option<String>,
    pub instance: Option<String>,
    pub format: OutputFormat,
    pub color: ColorMode,
    pub wait: Option<String>,
    pub timeout: Option<String>,
    pub yes: bool,
}

impl Default for CliGlobalOptions {
    fn default() -> Self {
        Self {
            profile: None,
            instance: None,
            format: OutputFormat::Human,
            color: ColorMode::Auto,
            wait: None,
            timeout: None,
            yes: false,
        }
    }
}