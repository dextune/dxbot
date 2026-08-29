//! Owner-verified, bounded Unix-domain local control client.
//!
//! Endpoint metadata is validated before every connection. Runtime identity,
//! HostGeneration and the server-derived local Principal are then bound by the
//! versioned handshake before preflight, query, watch, host action, submit or recovery lookup.

use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use application_contract::{
    LOCAL_CONTROL_PROTOCOL_VERSION, LOCAL_CONTROL_SCHEMA_VERSION, LocalControlHandshake,
    LocalControlHello, LocalControlRequest, LocalControlResponse, read_local_control_frame,
    write_local_control_frame,
};
use dxbot_core::types::{
    CanonicalTarget, CasConditions, CommandId, CommandPayload, IdempotencyKey, InstanceId,
    OperationRequest, OperationResult, PrincipalRef,
};
use serde_json::Value;

use crate::{ClientError, Transport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalControlClient {
    endpoint_path: PathBuf,
    instance_id: InstanceId,
    host_generation: i64,
    expected_owner_uid: u32,
    io_timeout: Duration,
}

impl LocalControlClient {
    pub fn new(
        endpoint_path: PathBuf,
        instance_id: InstanceId,
        host_generation: i64,
        expected_owner_uid: u32,
        io_timeout: Duration,
    ) -> Result<Self, ClientError> {
        if host_generation <= 0 {
            return Err(ClientError::Transport(
                "host generation must be positive".to_owned(),
            ));
        }
        if io_timeout.is_zero() {
            return Err(ClientError::Transport(
                "local control timeout must be non-zero".to_owned(),
            ));
        }
        let client = Self {
            endpoint_path,
            instance_id,
            host_generation,
            expected_owner_uid,
            io_timeout,
        };
        client.validate_endpoint()?;
        Ok(client)
    }

    pub fn endpoint_path(&self) -> &Path {
        &self.endpoint_path
    }

    pub fn handshake(&self) -> Result<LocalControlHandshake, ClientError> {
        let (_stream, handshake) = self.connect_and_handshake()?;
        Ok(handshake)
    }

    pub fn preflight(
        &self,
        payload: &CommandPayload,
        raw_selector: Option<Value>,
    ) -> Result<(CanonicalTarget, CasConditions), ClientError> {
        let (mut stream, handshake) = self.connect_and_handshake()?;
        validate_payload_identity(payload, &handshake)?;
        write_local_control_frame(
            &mut stream,
            &LocalControlRequest::Preflight {
                payload: payload.clone(),
                raw_selector,
            },
        )
        .map_err(codec_error)?;
        match read_local_control_frame::<_, LocalControlResponse>(&mut stream)
            .map_err(codec_error)?
        {
            LocalControlResponse::Preflight {
                canonical_target,
                cas,
            } => Ok((canonical_target, cas)),
            LocalControlResponse::Error { error } => Err(ClientError::Remote(Box::new(error))),
            _ => Err(ClientError::Transport(
                "unexpected response after preflight".to_owned(),
            )),
        }
    }

    pub fn query(&self, payload: &CommandPayload) -> Result<Value, ClientError> {
        let (mut stream, handshake) = self.connect_and_handshake()?;
        validate_payload_identity(payload, &handshake)?;
        write_local_control_frame(
            &mut stream,
            &LocalControlRequest::Query {
                payload: payload.clone(),
            },
        )
        .map_err(codec_error)?;
        match read_local_control_frame::<_, LocalControlResponse>(&mut stream)
            .map_err(codec_error)?
        {
            LocalControlResponse::Data { value } => Ok(value),
            LocalControlResponse::Error { error } => Err(ClientError::Remote(Box::new(error))),
            _ => Err(ClientError::Transport(
                "unexpected response after query".to_owned(),
            )),
        }
    }

    pub fn watch_next(
        &self,
        payload: &CommandPayload,
        cursor: Option<String>,
        timeout: Duration,
    ) -> Result<Value, ClientError> {
        let (mut stream, handshake) = self.connect_and_handshake()?;
        validate_payload_identity(payload, &handshake)?;
        let timeout_ms = u64::try_from(timeout.as_millis()).map_err(|_| {
            ClientError::Transport("watch timeout exceeds u64 milliseconds".to_owned())
        })?;
        write_local_control_frame(
            &mut stream,
            &LocalControlRequest::WatchNext {
                payload: payload.clone(),
                cursor,
                timeout_ms,
            },
        )
        .map_err(codec_error)?;
        match read_local_control_frame::<_, LocalControlResponse>(&mut stream)
            .map_err(codec_error)?
        {
            LocalControlResponse::Data { value } => Ok(value),
            LocalControlResponse::Error { error } => Err(ClientError::Remote(Box::new(error))),
            _ => Err(ClientError::Transport(
                "unexpected response after watch".to_owned(),
            )),
        }
    }

    pub fn stop_host(&self, host_generation: i64) -> Result<Value, ClientError> {
        let (mut stream, handshake) = self.connect_and_handshake()?;
        if host_generation != handshake.host_generation {
            return Err(ClientError::Transport(format!(
                "host generation mismatch: expected {}, got {host_generation}",
                handshake.host_generation
            )));
        }
        write_local_control_frame(
            &mut stream,
            &LocalControlRequest::StopHost { host_generation },
        )
        .map_err(codec_error)?;
        match read_local_control_frame::<_, LocalControlResponse>(&mut stream)
            .map_err(codec_error)?
        {
            LocalControlResponse::Data { value } => Ok(value),
            LocalControlResponse::Error { error } => Err(ClientError::Remote(Box::new(error))),
            _ => Err(ClientError::Transport(
                "unexpected response after host stop".to_owned(),
            )),
        }
    }

    pub fn into_transport(self) -> Transport {
        Box::new(move |request| self.submit(request))
    }

    pub fn submit(&self, request: &OperationRequest) -> Result<OperationResult, ClientError> {
        let (mut stream, handshake) = self.connect_and_handshake()?;
        if request.payload.instance_id != handshake.instance_id {
            return Err(ClientError::InstanceMismatch {
                client: handshake.instance_id,
                request: request.payload.instance_id.clone(),
            });
        }
        if request.payload.principal_ref != handshake.principal_ref
            || request.idempotency_key.principal_ref != handshake.principal_ref
        {
            return Err(ClientError::Transport(
                "request principal does not match authenticated local principal".to_owned(),
            ));
        }
        write_local_control_frame(
            &mut stream,
            &LocalControlRequest::Submit {
                request: request.clone(),
            },
        )
        .map_err(codec_error)?;
        match read_local_control_frame::<_, LocalControlResponse>(&mut stream)
            .map_err(codec_error)?
        {
            LocalControlResponse::Operation { result } => Ok(result),
            LocalControlResponse::Error { error } => Err(ClientError::Remote(Box::new(error))),
            _ => Err(ClientError::Transport(
                "unexpected response after operation submit".to_owned(),
            )),
        }
    }

    pub fn lookup_binding(
        &self,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Result<Option<OperationResult>, ClientError> {
        let (mut stream, handshake) = self.connect_and_handshake()?;
        if key.principal_ref != handshake.principal_ref {
            return Err(ClientError::Transport(
                "lookup key principal does not match authenticated local principal".to_owned(),
            ));
        }
        write_local_control_frame(
            &mut stream,
            &LocalControlRequest::LookupBinding {
                command_id: command_id.clone(),
                idempotency_key: key.clone(),
            },
        )
        .map_err(codec_error)?;
        match read_local_control_frame::<_, LocalControlResponse>(&mut stream)
            .map_err(codec_error)?
        {
            LocalControlResponse::Binding { result } => Ok(result),
            LocalControlResponse::Error { error } => Err(ClientError::Remote(Box::new(error))),
            _ => Err(ClientError::Transport(
                "unexpected response after binding lookup".to_owned(),
            )),
        }
    }

    fn connect_and_handshake(&self) -> Result<(UnixStream, LocalControlHandshake), ClientError> {
        self.validate_endpoint()?;
        let mut stream = UnixStream::connect(&self.endpoint_path)
            .map_err(|error| transport_io("connect", error))?;
        stream
            .set_read_timeout(Some(self.io_timeout))
            .map_err(|error| transport_io("set read timeout", error))?;
        stream
            .set_write_timeout(Some(self.io_timeout))
            .map_err(|error| transport_io("set write timeout", error))?;
        write_local_control_frame(
            &mut stream,
            &LocalControlRequest::Hello {
                hello: LocalControlHello::new(self.instance_id.clone(), self.host_generation),
            },
        )
        .map_err(codec_error)?;
        let response = read_local_control_frame::<_, LocalControlResponse>(&mut stream)
            .map_err(codec_error)?;
        let handshake = match response {
            LocalControlResponse::Handshake { handshake } => handshake,
            LocalControlResponse::Error { error } => {
                return Err(ClientError::Remote(Box::new(error)));
            }
            _ => {
                return Err(ClientError::Transport(
                    "non-handshake response received before handshake".to_owned(),
                ));
            }
        };
        self.validate_handshake(&handshake)?;
        Ok((stream, handshake))
    }

    fn validate_handshake(&self, handshake: &LocalControlHandshake) -> Result<(), ClientError> {
        if handshake.instance_id != self.instance_id {
            return Err(ClientError::Transport(format!(
                "handshake instance mismatch: expected {}, got {}",
                self.instance_id.0, handshake.instance_id.0
            )));
        }
        if handshake.host_generation != self.host_generation {
            return Err(ClientError::Transport(format!(
                "handshake host generation mismatch: expected {}, got {}",
                self.host_generation, handshake.host_generation
            )));
        }
        if handshake.protocol_version != LOCAL_CONTROL_PROTOCOL_VERSION
            || handshake.schema_version != LOCAL_CONTROL_SCHEMA_VERSION
        {
            return Err(ClientError::Transport(format!(
                "incompatible local control version: protocol={}, schema={}",
                handshake.protocol_version, handshake.schema_version
            )));
        }
        validate_principal(
            &self.instance_id,
            &handshake.principal_ref,
            self.expected_owner_uid,
        )
    }

    fn validate_endpoint(&self) -> Result<(), ClientError> {
        let metadata = fs::symlink_metadata(&self.endpoint_path)
            .map_err(|error| transport_io("stat endpoint", error))?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() || !file_type.is_socket() {
            return Err(ClientError::Transport(format!(
                "control endpoint is not a direct Unix socket: {}",
                self.endpoint_path.display()
            )));
        }
        if metadata.uid() != self.expected_owner_uid {
            return Err(ClientError::Transport(format!(
                "control endpoint owner mismatch: expected uid {}, got {}",
                self.expected_owner_uid,
                metadata.uid()
            )));
        }
        let mode = metadata.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(ClientError::Transport(format!(
                "control endpoint permissions are not owner-only: {mode:o}"
            )));
        }
        Ok(())
    }
}

fn validate_payload_identity(
    payload: &CommandPayload,
    handshake: &LocalControlHandshake,
) -> Result<(), ClientError> {
    if payload.instance_id != handshake.instance_id {
        return Err(ClientError::InstanceMismatch {
            client: handshake.instance_id.clone(),
            request: payload.instance_id.clone(),
        });
    }
    if payload.principal_ref != handshake.principal_ref {
        return Err(ClientError::Transport(
            "payload principal does not match authenticated local principal".to_owned(),
        ));
    }
    Ok(())
}

fn validate_principal(
    instance_id: &InstanceId,
    principal_ref: &PrincipalRef,
    owner_uid: u32,
) -> Result<(), ClientError> {
    let expected = format!("local:{}:uid:{owner_uid}", instance_id.0);
    if principal_ref.0 == expected {
        Ok(())
    } else {
        Err(ClientError::Transport(format!(
            "authenticated principal mismatch: expected {expected}, got {}",
            principal_ref.0
        )))
    }
}

fn codec_error(error: application_contract::LocalControlCodecError) -> ClientError {
    ClientError::Transport(error.to_string())
}

fn transport_io(action: &str, error: std::io::Error) -> ClientError {
    ClientError::Transport(format!("{action}: {error}"))
}
