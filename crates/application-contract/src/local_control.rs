//! Versioned local control wire contract shared by CLI client and Runtime
//! control server.
//!
//! Transport framing is a 32-bit big-endian byte length followed by one JSON
//! document. The frame ceiling is part of the contract so malformed local peers
//! cannot force unbounded allocation before authentication/dispatch.

use std::fmt;
use std::io::{Read, Write};

use dxbot_core::DxbotError;
use dxbot_core::types::{InstanceId, OperationRequest, OperationResult, PrincipalRef};
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
    Submit { request: OperationRequest },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum LocalControlResponse {
    Handshake { handshake: LocalControlHandshake },
    Operation { result: OperationResult },
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
            Self::InvalidJson(message) => {
                write!(formatter, "invalid local control JSON: {message}")
            }
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

/// Write one bounded JSON frame.
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
    let length = u32::try_from(payload.len()).map_err(|_| {
        LocalControlCodecError::OversizedFrame {
            length: payload.len(),
            maximum: MAX_LOCAL_CONTROL_FRAME_BYTES,
        }
    })?;
    writer
        .write_all(&length.to_be_bytes())
        .and_then(|()| writer.write_all(&payload))
        .and_then(|()| writer.flush())
        .map_err(|error| LocalControlCodecError::Io(error.to_string()))
}

/// Read exactly one bounded JSON frame. The advertised length is validated
/// before allocating the payload buffer.
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
    fn oversized_advertised_frame_is_rejected_before_payload_read() {
        let oversized = (MAX_LOCAL_CONTROL_FRAME_BYTES as u32 + 1).to_be_bytes();
        let error = read_local_control_frame::<_, LocalControlRequest>(&mut oversized.as_slice())
            .expect_err("oversized frame must fail");
        assert!(matches!(
            error,
            LocalControlCodecError::OversizedFrame { .. }
        ));
    }
}
