CREATE TABLE runtime_metadata (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    instance_id TEXT NOT NULL,
    host_generation INTEGER NOT NULL
);
CREATE TABLE aggregate_state (
    aggregate_id TEXT PRIMARY KEY,
    revision INTEGER NOT NULL,
    state_value TEXT NOT NULL,
    last_operation_id TEXT NOT NULL
);
CREATE TABLE events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    payload TEXT NOT NULL
);
CREATE TABLE outbox (
    operation_id TEXT PRIMARY KEY,
    payload TEXT NOT NULL
);
CREATE TABLE receipts (
    operation_id TEXT PRIMARY KEY,
    disposition TEXT NOT NULL,
    result_ref TEXT NOT NULL,
    resolved_binding_digest TEXT NOT NULL
);
CREATE TABLE command_bindings (
    command_id TEXT PRIMARY KEY,
    principal_ref TEXT NOT NULL,
    key_digest TEXT NOT NULL,
    request_digest TEXT NOT NULL,
    operation_id TEXT NOT NULL UNIQUE,
    terminal_disposition TEXT,
    expires_at INTEGER NOT NULL,
    compacted INTEGER NOT NULL DEFAULT 0 CHECK (compacted IN (0, 1))
);
CREATE TABLE principal_bindings (
    principal_ref TEXT NOT NULL,
    key_digest TEXT NOT NULL,
    command_id TEXT NOT NULL,
    request_digest TEXT NOT NULL,
    operation_id TEXT NOT NULL UNIQUE,
    terminal_disposition TEXT,
    expires_at INTEGER NOT NULL,
    compacted INTEGER NOT NULL DEFAULT 0 CHECK (compacted IN (0, 1)),
    PRIMARY KEY (principal_ref, key_digest),
    FOREIGN KEY (command_id) REFERENCES command_bindings(command_id) ON DELETE CASCADE
);
CREATE TABLE audit_intents (
    operation_id TEXT PRIMARY KEY,
    payload TEXT NOT NULL
);

INSERT INTO runtime_metadata(singleton, instance_id, host_generation)
VALUES (1, 'instance-1', 1);
INSERT INTO aggregate_state(aggregate_id, revision, state_value, last_operation_id)
VALUES ('bot-old', 1, 'state-old', 'operation-old');
INSERT INTO events(operation_id, aggregate_id, payload)
VALUES ('operation-old', 'bot-old', 'event-old');
INSERT INTO outbox(operation_id, payload)
VALUES ('operation-old', 'outbox-old');
INSERT INTO receipts(operation_id, disposition, result_ref, resolved_binding_digest)
VALUES ('operation-old', 'accepted', 'result-old', 'binding-old');
INSERT INTO command_bindings(
    command_id, principal_ref, key_digest, request_digest, operation_id,
    terminal_disposition, expires_at, compacted
) VALUES (
    'command-old', 'principal-a', 'key-old', 'request-old', 'operation-old',
    NULL, 100, 0
);
INSERT INTO principal_bindings(
    principal_ref, key_digest, command_id, request_digest, operation_id,
    terminal_disposition, expires_at, compacted
) VALUES (
    'principal-a', 'key-old', 'command-old', 'request-old', 'operation-old',
    NULL, 100, 0
);
INSERT INTO audit_intents(operation_id, payload)
VALUES ('operation-old', 'audit-old');

PRAGMA user_version = 1;
