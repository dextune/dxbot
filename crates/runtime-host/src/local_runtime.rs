//! Single-writer local Runtime composition and verified control endpoint
//! publication.
//!
//! This is the Runtime Host owner for first-Instance bootstrap, generation
//! fencing, stale endpoint replacement, control-server composition, and
//! discovery publication. The CLI only spawns/contacts this owner.

#![cfg(unix)]

use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use application::ApplicationMutator;
use control_server::{ControlServer, LocalControlServer, SecurityState};
use dxbot_core::types::InstanceId;
use fs2::FileExt;
use runtime_bootstrap::bootstrap::Error as BootstrapError;
use runtime_bootstrap::{DiscoveryEndpoint, DiscoveryState, RuntimeBootstrap};

const HOST_LOCK_FILE: &str = ".runtime-host.lock";
const OWNER_DIRECTORY_MODE: u32 = 0o700;
const OWNER_FILE_MODE: u32 = 0o600;

#[derive(Debug)]
pub struct LocalRuntimeHost {
    _host_lock: File,
    runtime_root: PathBuf,
    discovery_root: PathBuf,
    instance_id: InstanceId,
    host_generation: i64,
    endpoint_uri: String,
    server: LocalControlServer,
}

impl LocalRuntimeHost {
    pub fn start(
        runtime_root: PathBuf,
        discovery_root: PathBuf,
        profile: Option<String>,
    ) -> Result<Self, io::Error> {
        fs::create_dir_all(&runtime_root)?;
        fs::set_permissions(
            &runtime_root,
            fs::Permissions::from_mode(OWNER_DIRECTORY_MODE),
        )?;
        let host_lock_path = runtime_root.join(HOST_LOCK_FILE);
        let host_lock = open_host_lock(&host_lock_path)?;
        host_lock.try_lock_exclusive().map_err(|error| {
            io::Error::new(
                io::ErrorKind::WouldBlock,
                format!(
                    "another Runtime Host owns {}: {error}",
                    runtime_root.display()
                ),
            )
        })?;

        let mut bootstrap = RuntimeBootstrap::new();
        let (instance_id, host_generation) = match bootstrap.bootstrap_first_instance(&runtime_root)
        {
            Ok(instance_id) => (instance_id, 1),
            Err(BootstrapError::AlreadyBootstrapped) => {
                let instance_id = bootstrap
                    .attach_existing(&runtime_root)
                    .map_err(bootstrap_io)?;
                let generation = bootstrap.advance_host_generation().map_err(bootstrap_io)?;
                (instance_id, generation)
            }
            Err(error) => return Err(bootstrap_io(error)),
        };
        let endpoint_path = bootstrap
            .endpoint_path(&instance_id)
            .map_err(bootstrap_io)?;
        replace_bootstrap_or_stale_endpoint(&endpoint_path)?;

        let application = Arc::new(ApplicationMutator::new());
        let security = Arc::new(Mutex::new(SecurityState::new()));
        let control = Arc::new(ControlServer::new(security, application));
        let server = LocalControlServer::bind(
            endpoint_path.clone(),
            instance_id.clone(),
            host_generation,
            control,
        )?;
        let endpoint_uri = endpoint_uri(&endpoint_path)?;

        publish_discovery(
            &discovery_root,
            DiscoveryEndpoint {
                instance_id: instance_id.clone(),
                profile,
                endpoint: endpoint_uri.clone(),
                host_generation,
                provider_id: "unconfigured".to_owned(),
                provider_ready: false,
            },
        )?;

        Ok(Self {
            _host_lock: host_lock,
            runtime_root,
            discovery_root,
            instance_id,
            host_generation,
            endpoint_uri,
            server,
        })
    }

    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    pub fn host_generation(&self) -> i64 {
        self.host_generation
    }

    pub fn endpoint_uri(&self) -> &str {
        &self.endpoint_uri
    }

    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }

    pub fn serve(&self) -> Result<(), io::Error> {
        self.server.serve()
    }

    pub fn serve_one(&self) -> Result<(), io::Error> {
        self.server.serve_one()
    }
}

impl Drop for LocalRuntimeHost {
    fn drop(&mut self) {
        let Ok(mut state) = DiscoveryState::load_state(&self.discovery_root) else {
            return;
        };
        let should_remove = state
            .instance_endpoints
            .get(&self.instance_id)
            .is_some_and(|endpoint| {
                endpoint.host_generation == self.host_generation
                    && endpoint.endpoint == self.endpoint_uri
            });
        if should_remove {
            state.instance_endpoints.remove(&self.instance_id);
            let _ = state.save_state(&self.discovery_root);
        }
    }
}

fn open_host_lock(path: &Path) -> Result<File, io::Error> {
    let file = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("Runtime Host lock is not a direct regular file: {}", path.display()),
                ));
            }
            OpenOptions::new().read(true).write(true).open(path)?
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)?,
        Err(error) => return Err(error),
    };
    fs::set_permissions(path, fs::Permissions::from_mode(OWNER_FILE_MODE))?;
    Ok(file)
}

fn publish_discovery(root: &Path, endpoint: DiscoveryEndpoint) -> Result<(), io::Error> {
    let mut state = DiscoveryState::load_state(root).map_err(bootstrap_io)?;
    state
        .instance_endpoints
        .insert(endpoint.instance_id.clone(), endpoint);
    state.save_state(root).map_err(bootstrap_io)
}

fn replace_bootstrap_or_stale_endpoint(path: &Path) -> Result<(), io::Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("refusing to replace symlink endpoint: {}", path.display()),
                ));
            }
            if !file_type.is_file() && !file_type.is_socket() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("refusing to replace unexpected endpoint type: {}", path.display()),
                ));
            }
            fs::remove_file(path)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn endpoint_uri(path: &Path) -> Result<String, io::Error> {
    let path = path.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "local control endpoint path is not valid UTF-8",
        )
    })?;
    Ok(format!("unix://{path}"))
}

fn bootstrap_io(error: BootstrapError) -> io::Error {
    match error {
        BootstrapError::AlreadyBootstrapped => io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Runtime Instance already bootstrapped",
        ),
        BootstrapError::InstanceNotFound => {
            io::Error::new(io::ErrorKind::NotFound, "Runtime Instance not found")
        }
        BootstrapError::NoStateRoot => io::Error::new(
            io::ErrorKind::InvalidInput,
            "Runtime state root is unavailable",
        ),
        BootstrapError::CorruptState(message) | BootstrapError::Io(message) => {
            io::Error::other(message)
        }
    }
}
