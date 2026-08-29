//! Owner-only Unix-domain control transport.
//!
//! P0 LocalPrincipal is derived server-side from the Runtime Instance and the
//! authenticated peer UID. The socket itself is owner-only and clients verify
//! its owner/type/generation, but pathname permissions are not treated as peer
//! identity. Same-UID process isolation is intentionally not claimed.
//! Every connection performs a version/generation handshake before accepting one
//! bounded request.

#![cfg(unix)]

use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use application_contract::{
    LOCAL_CONTROL_PROTOCOL_VERSION, LOCAL_CONTROL_SCHEMA_VERSION, LocalControlHandshake,
    LocalControlRequest, LocalControlResponse, read_local_control_frame, write_local_control_frame,
};
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::{InstanceId, PrincipalRef};

use crate::server::ControlServer;

const SOCKET_MODE: u32 = 0o600;
const IO_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct LocalControlServer {
    listener: UnixListener,
    endpoint_path: PathBuf,
    endpoint_dev: u64,
    endpoint_ino: u64,
    instance_id: InstanceId,
    host_generation: i64,
    owner_principal_ref: PrincipalRef,
    control: Arc<ControlServer>,
}

impl LocalControlServer {
    pub fn bind(
        endpoint_path: PathBuf,
        instance_id: InstanceId,
        host_generation: i64,
        control: Arc<ControlServer>,
    ) -> Result<Self, io::Error> {
        if host_generation <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "host generation must be positive",
            ));
        }
        validate_absent_endpoint(&endpoint_path)?;
        if let Some(parent) = endpoint_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(&endpoint_path)?;
        fs::set_permissions(&endpoint_path, fs::Permissions::from_mode(SOCKET_MODE))?;
        let metadata = fs::symlink_metadata(&endpoint_path)?;
        if !metadata.file_type().is_socket() || metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "bound local control endpoint is not a direct Unix socket",
            ));
        }
        if metadata.permissions().mode() & 0o777 != SOCKET_MODE {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "local control endpoint is not owner-only",
            ));
        }

        let owner_principal_ref = local_principal(&instance_id, metadata.uid());
        control
            .register_local_operator(&owner_principal_ref)
            .map_err(|error| io::Error::other(error.to_string()))?;

        Ok(Self {
            listener,
            endpoint_path,
            endpoint_dev: metadata.dev(),
            endpoint_ino: metadata.ino(),
            instance_id,
            host_generation,
            owner_principal_ref,
            control,
        })
    }

    /// Principal expected for the owner UID. Actual requests always use the
    /// peer credential read from each accepted socket.
    pub fn principal_ref(&self) -> &PrincipalRef {
        &self.owner_principal_ref
    }

    pub fn endpoint_path(&self) -> &Path {
        &self.endpoint_path
    }

    pub fn serve(&self) -> Result<(), io::Error> {
        loop {
            let (stream, _) = self.listener.accept()?;
            let _ = self.handle_stream(stream);
        }
    }

    pub fn serve_one(&self) -> Result<(), io::Error> {
        let (stream, _) = self.listener.accept()?;
        self.handle_stream(stream)
    }

    fn handle_stream(&self, mut stream: UnixStream) -> Result<(), io::Error> {
        stream.set_read_timeout(Some(IO_TIMEOUT))?;
        stream.set_write_timeout(Some(IO_TIMEOUT))?;
        let authenticated_principal = local_principal(&self.instance_id, peer_uid(&stream)?);

        let first = read_local_control_frame::<_, LocalControlRequest>(&mut stream)
            .map_err(codec_io)?;
        let hello = match first {
            LocalControlRequest::Hello { hello } => hello,
            LocalControlRequest::Submit { .. } | LocalControlRequest::LookupBinding { .. } => {
                write_error(
                    &mut stream,
                    protocol_error("request received before local control handshake"),
                )?;
                return Ok(());
            }
        };

        if hello.instance_id != self.instance_id
            || hello.host_generation != self.host_generation
            || hello.protocol_version != LOCAL_CONTROL_PROTOCOL_VERSION
            || hello.schema_version != LOCAL_CONTROL_SCHEMA_VERSION
        {
            write_error(
                &mut stream,
                incompatible_error(format!(
                    "local control handshake mismatch: instance={}, generation={}, protocol={}, schema={}",
                    hello.instance_id.0,
                    hello.host_generation,
                    hello.protocol_version,
                    hello.schema_version
                )),
            )?;
            return Ok(());
        }

        write_local_control_frame(
            &mut stream,
            &LocalControlResponse::Handshake {
                handshake: LocalControlHandshake {
                    instance_id: self.instance_id.clone(),
                    host_generation: self.host_generation,
                    principal_ref: authenticated_principal.clone(),
                    protocol_version: LOCAL_CONTROL_PROTOCOL_VERSION.to_owned(),
                    schema_version: LOCAL_CONTROL_SCHEMA_VERSION.to_owned(),
                },
            },
        )
        .map_err(codec_io)?;

        let second = read_local_control_frame::<_, LocalControlRequest>(&mut stream)
            .map_err(codec_io)?;
        match second {
            LocalControlRequest::Hello { .. } => write_error(
                &mut stream,
                protocol_error("duplicate local control handshake"),
            ),
            LocalControlRequest::Submit { request } => {
                if request.payload.instance_id != self.instance_id {
                    return write_error(
                        &mut stream,
                        incompatible_error("request instance does not match endpoint".to_owned()),
                    );
                }
                match self
                    .control
                    .handle_request(&authenticated_principal, &request)
                {
                    Ok(result) => write_local_control_frame(
                        &mut stream,
                        &LocalControlResponse::Operation { result },
                    )
                    .map_err(codec_io),
                    Err(error) => write_error(&mut stream, error.to_dxbot_error()),
                }
            }
            LocalControlRequest::LookupBinding {
                command_id,
                idempotency_key,
            } => match self.control.lookup_binding(
                &authenticated_principal,
                &command_id,
                &idempotency_key,
            ) {
                Ok(result) => write_local_control_frame(
                    &mut stream,
                    &LocalControlResponse::Binding { result },
                )
                .map_err(codec_io),
                Err(error) => write_error(&mut stream, error.to_dxbot_error()),
            },
        }
    }
}

impl Drop for LocalControlServer {
    fn drop(&mut self) {
        let Ok(metadata) = fs::symlink_metadata(&self.endpoint_path) else {
            return;
        };
        if metadata.file_type().is_socket()
            && metadata.dev() == self.endpoint_dev
            && metadata.ino() == self.endpoint_ino
        {
            let _ = fs::remove_file(&self.endpoint_path);
        }
    }
}

fn local_principal(instance_id: &InstanceId, uid: u32) -> PrincipalRef {
    PrincipalRef(format!("local:{}:uid:{uid}", instance_id.0))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn peer_uid(stream: &UnixStream) -> Result<u32, io::Error> {
    let credentials = nix::sys::socket::getsockopt(
        stream,
        nix::sys::socket::sockopt::PeerCredentials,
    )
    .map_err(|error| io::Error::other(format!("cannot authenticate local peer: {error}")))?;
    Ok(credentials.uid())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn peer_uid(_stream: &UnixStream) -> Result<u32, io::Error> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "safe peer-credential authentication is not implemented for this Unix target",
    ))
}

fn validate_absent_endpoint(path: &Path) -> Result<(), io::Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let kind = if metadata.file_type().is_symlink() {
                "symlink"
            } else if metadata.file_type().is_socket() {
                "socket"
            } else {
                "non-socket file"
            };
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "refusing to replace existing control endpoint {kind}: {}",
                    path.display()
                ),
            ))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn write_error(stream: &mut UnixStream, error: DxbotError) -> Result<(), io::Error> {
    write_local_control_frame(stream, &LocalControlResponse::Error { error }).map_err(codec_io)
}

fn protocol_error(message: impl Into<String>) -> DxbotError {
    dxbot_error(
        ErrorCode::InvalidInput,
        ErrorCategory::Input,
        message.into(),
    )
}

fn incompatible_error(message: impl Into<String>) -> DxbotError {
    dxbot_error(
        ErrorCode::Incompatible,
        ErrorCategory::Conflict,
        message.into(),
    )
}

fn dxbot_error(code: ErrorCode, category: ErrorCategory, message: String) -> DxbotError {
    DxbotError {
        code,
        category,
        message,
        retryable: false,
        operation_ref: None,
        target_refs: Vec::new(),
        field_violations: Vec::new(),
        current_revision: None,
        current_generation: None,
        resume_cursor: None,
        next_actions: Vec::new(),
    }
}

fn codec_io(error: application_contract::LocalControlCodecError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}
