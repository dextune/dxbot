//! Versioned bounded local control contract shared by CLI and Runtime.

use std::fmt;
use std::io::{Read, Write};

use dxbot_core::DxbotError;
use dxbot_core::types::{
    CanonicalTarget, CasConditions, CommandId, CommandPayload, IdempotencyKey, InstanceId,
    OperationRequest, OperationResult, PrincipalRef,
};
use serde::de::DeserializeOwned;
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
    Preflight {
        payload: CommandPayload,
        raw_selector: Option<serde_json::Value>,
    },
    Query { payload: CommandPayload },
    WatchNext {
        payload: CommandPayload,
        cursor: Option<String>,
        timeout_ms: u64,
    },
    Submit { request: OperationRequest },
    LookupBinding {
        command_id: CommandId,
        idempotency_key: IdempotencyKey,
    },
    StopHost { host_generation: i64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum LocalControlResponse {
    Handshake { handshake: LocalControlHandshake },
    Preflight {
        canonical_target: CanonicalTarget,
        cas: CasConditions,
    },
    Data { value: serde_json::Value },
    Operation { result: OperationResult },
    Binding { result: Option<OperationResult> },
    Error { error: DxbotError },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalControlCodecError {
    Io(String),
    EmptyFrame,
    OversizedFrame { length: usize, maximum: usize },
    InvalidJson(String),
}

impl fmt::Display for LocalControlCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "local control I/O error: {message}"),
            Self::EmptyFrame => write!(formatter, "local control frame is empty"),
            Self::OversizedFrame { length, maximum } => write!(
                formatter,
                "local control frame exceeds bound: {length} > {maximum}"
            ),
            Self::InvalidJson(message) => write!(formatter, "invalid local control JSON: {message}"),
        }
    }
}

impl std::error::Error for LocalControlCodecError {}

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

pub fn write_local_control_frame<W, T>(
    writer: &mut W,
    value: &T,
) -> Result<(), LocalControlCodecError>
where
    W: Write,
    T: Serialize,
{
    let payload = serde_json::to_vec(value)
        .map_err(|error| LocalControlCodecError::InvalidJson(error.to_string()))?;
    if payload.is_empty() {
        return Err(LocalControlCodecError::EmptyFrame);
    }
    if payload.len() > MAX_LOCAL_CONTROL_FRAME_BYTES {
        return Err(LocalControlCodecError::OversizedFrame {
            length: payload.len(),
            maximum: MAX_LOCAL_CONTROL_FRAME_BYTES,
        });
    }
    let length = u32::try_from(payload.len()).map_err(|_| LocalControlCodecError::OversizedFrame {
        length: payload.len(),
        maximum: MAX_LOCAL_CONTROL_FRAME_BYTES,
    })?;
    writer
        .write_all(&length.to_be_bytes())
        .and_then(|()| writer.write_all(&payload))
        .and_then(|()| writer.flush())
        .map_err(|error| LocalControlCodecError::Io(error.to_string()))
}

pub fn read_local_control_frame<R, T>(reader: &mut R) -> Result<T, LocalControlCodecError>
where
    R: Read,
    T: DeserializeOwned,
{
    let mut header = [0_u8; 4];
    reader
        .read_exact(&mut header)
        .map_err(|error| LocalControlCodecError::Io(error.to_string()))?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 {
        return Err(LocalControlCodecError::EmptyFrame);
    }
    if length > MAX_LOCAL_CONTROL_FRAME_BYTES {
        return Err(LocalControlCodecError::OversizedFrame {
            length,
            maximum: MAX_LOCAL_CONTROL_FRAME_BYTES,
        });
    }
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .map_err(|error| LocalControlCodecError::Io(error.to_string()))?;
    serde_json::from_slice(&payload)
        .map_err(|error| LocalControlCodecError::InvalidJson(error.to_string()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn framed_roundtrip_preserves_hello() {
        let request = LocalControlRequest::Hello {
            hello: LocalControlHello::new(InstanceId("instance-a".to_owned()), 7),
        };
        let mut bytes = Vec::new();
        write_local_control_frame(&mut bytes, &request).expect("frame writes");
        let decoded: LocalControlRequest =
            read_local_control_frame(&mut bytes.as_slice()).expect("frame reads");
        assert_eq!(decoded, request);
    }

    #[test]
    fn framed_roundtrip_preserves_binding_lookup() {
        let request = LocalControlRequest::LookupBinding {
            command_id: CommandId("command-a".to_owned()),
            idempotency_key: IdempotencyKey {
                principal_ref: PrincipalRef("principal-a".to_owned()),
                key_digest: "key-a".to_owned(),
                expires_at: 7,
            },
        };
        let mut bytes = Vec::new();
        write_local_control_frame(&mut bytes, &request).expect("frame writes");
        let decoded: LocalControlRequest =
            read_local_control_frame(&mut bytes.as_slice()).expect("frame reads");
        assert_eq!(decoded, request);
    }

    #[test]
    fn watch_request_roundtrips() {
        let request = LocalControlRequest::WatchNext {
            payload: CommandPayload {
                command_key: "task-watch".to_owned(),
                principal_ref: PrincipalRef("principal".to_owned()),
                instance_id: InstanceId("instance".to_owned()),
                canonical_target: CanonicalTarget::Task {
                    id: dxbot_core::types::TaskId("task".to_owned()),
                    revision: 1,
                    execution_generation: Some(1),
                },
                cas: None,
                content: None,
                semantic_options: serde_json::json!({}),
            },
            cursor: Some("1".to_owned()),
            timeout_ms: 1000,
        };
        let mut bytes = Vec::new();
        write_local_control_frame(&mut bytes, &request).expect("frame writes");
        let decoded: LocalControlRequest =
            read_local_control_frame(&mut bytes.as_slice()).expect("frame reads");
        assert_eq!(decoded, request);
    }

    #[test]
    fn host_stop_generation_roundtrips() {
        let request = LocalControlRequest::StopHost { host_generation: 9 };
        let mut bytes = Vec::new();
        write_local_control_frame(&mut bytes, &request).expect("frame writes");
        let decoded: LocalControlRequest =
            read_local_control_frame(&mut bytes.as_slice()).expect("frame reads");
        assert_eq!(decoded, request);
    }

    #[test]
    fn oversized_advertised_frame_is_rejected_before_payload_read() {
        let oversized = (MAX_LOCAL_CONTROL_FRAME_BYTES as u32 + 1).to_be_bytes();
        let error = read_local_control_frame::<_, LocalControlRequest>(&mut oversized.as_slice())
            .expect_err("oversized frame must fail");
        assert!(matches!(error, LocalControlCodecError::OversizedFrame { .. }));
    }
}
