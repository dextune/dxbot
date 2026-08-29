//! Versioned local control wire contract shared by CLI client and Runtime
//! control server.
//!
//! Transport framing is a 32-bit big-endian byte length followed by one JSON
//! document. The frame ceiling is part of the contract so malformed local peers
//! cannot force unbounded allocation before authentication/dispatch.

use dxbot_core::DxbotError;
use dxbot_core::types::{InstanceId, OperationRequest, OperationResult, PrincipalRef};
use serde::{Deserialize, Serialize};

pub const LOCAL_CONTROL_PROTOCOL_VERSION: &str = "1";
pub const LOCAL_CONTROL_SCHEMA_VERSION: &str = "v1";
pub const MAX_LOCAL_CONTROL_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalControlHello {
    pub instance_id: InstanceId,
    pub host_generation: i64,
    pub protocol_version: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalControlHandshake {
    pub instance_id: InstanceId,
    pub host_generation: i64,
    pub principal_ref: PrincipalRef,
    pub protocol_version: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum LocalControlRequest {
    Hello { hello: LocalControlHello },
    Submit { request: OperationRequest },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum LocalControlResponse {
    Handshake { handshake: LocalControlHandshake },
    Operation { result: OperationResult },
    Error { error: DxbotError },
}

impl LocalControlHello {
    pub fn new(instance_id: InstanceId, host_generation: i64) -> Self {
        Self {
            instance_id,
            host_generation,
            protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_owned(),
            schema_version: LOCAL_CONTROL_SCHEMA_VERSION.to_owned(),
        }
    }
}
