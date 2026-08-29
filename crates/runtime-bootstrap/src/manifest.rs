//! Versioned first-init initialization manifest.
//!
//! First-init must atomically decide every default that the Runtime Host and
//! its canonical owners later depend on: the persistent `InstanceId`, the first
//! `HostGeneration`, the derived owner `LocalPrincipal` identity/binding
//! metadata, the default Brain/Permission/Resource/Provider policy generations,
//! and the persistent `DataSchemaVersion`.
//!
//! The manifest is an *initialization record*, not a second copy of canonical
//! Application/Security state. Bootstrap owns the decision of these defaults at
//! first init; the Runtime Host consumes the manifest to register the derived
//! local operator into the canonical Security `PrincipalManager`. Bootstrap
//! never mutates Security/Application state itself, so there is no duplicated
//! Canonical Owner.

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use dxbot_core::types::{InstanceId, PrincipalRef};
use serde::{Deserialize, Serialize};

use crate::bootstrap::Error;

/// Canonical file name of the first-init manifest inside a Runtime state root.
pub const INSTANCE_MANIFEST_FILE: &str = "instance-manifest.json";

/// Manifest schema version. Bumped only when the initialization record shape
/// changes in a way the Runtime Host must distinguish.
pub const INSTANCE_MANIFEST_VERSION: u32 = 1;

/// Persistent data schema version stamped at first init.
pub const DEFAULT_DATA_SCHEMA_VERSION: u32 = 1;

/// Initial value of every default policy generation created at first init.
pub const INITIAL_POLICY_GENERATION: i64 = 1;

/// Bound on manifest size to keep reads bounded and fail closed on corruption.
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

/// Default policy generations decided at first init. Each field is the current
/// generation of the corresponding default policy family; the canonical policy
/// owners advance these later. Bootstrap only records the initial values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefaultPolicyGenerations {
    pub brain: i64,
    pub permission: i64,
    pub resource: i64,
    pub provider: i64,
}

impl DefaultPolicyGenerations {
    fn initial() -> Self {
        Self {
            brain: INITIAL_POLICY_GENERATION,
            permission: INITIAL_POLICY_GENERATION,
            resource: INITIAL_POLICY_GENERATION,
            provider: INITIAL_POLICY_GENERATION,
        }
    }

    fn validate(&self) -> Result<(), Error> {
        for (name, value) in [
            ("brain", self.brain),
            ("permission", self.permission),
            ("resource", self.resource),
            ("provider", self.provider),
        ] {
            if value <= 0 {
                return Err(Error::CorruptState(format!(
                    "default {name} policy generation must be positive, found {value}"
                )));
            }
        }
        Ok(())
    }
}

/// Derived owner LocalPrincipal identity and its initial AuthorityBinding
/// metadata, decided at first init.
///
/// This is *initialization metadata* consumed by the Runtime Host to register
/// the operator into the canonical Security registry. It intentionally holds no
/// mutable security state (status, revocation, scoped grants) — those remain
/// owned by `runtime-security`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerInitialization {
    /// Server-derivable owner principal reference. On unix this binds the OS
    /// UID that ran first init, matching the `InstanceId + authenticated UID`
    /// LocalPrincipal derivation rule.
    pub principal_ref: PrincipalRef,
    /// OS UID that owns the Runtime state at first init, or `None` off unix.
    pub owner_uid: Option<u32>,
    /// Stable identity of the owner AuthorityBinding created at first init.
    pub authority_binding_id: String,
    /// Initial generation of the owner AuthorityBinding.
    pub authority_generation: i64,
    /// First-init timestamp in seconds since the Unix epoch.
    pub issued_at: i64,
}

impl OwnerInitialization {
    fn validate(&self) -> Result<(), Error> {
        if self.principal_ref.0.trim().is_empty() {
            return Err(Error::CorruptState(
                "owner principal reference is empty".to_owned(),
            ));
        }
        if self.authority_binding_id.trim().is_empty() {
            return Err(Error::CorruptState(
                "owner authority binding id is empty".to_owned(),
            ));
        }
        if self.authority_generation <= 0 {
            return Err(Error::CorruptState(format!(
                "owner authority generation must be positive, found {}",
                self.authority_generation
            )));
        }
        Ok(())
    }
}

/// The complete first-init initialization record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceManifest {
    pub manifest_version: u32,
    pub instance_id: InstanceId,
    pub host_generation: i64,
    pub data_schema_version: u32,
    pub owner: OwnerInitialization,
    pub policy_generations: DefaultPolicyGenerations,
}

impl InstanceManifest {
    /// Build the canonical first-init manifest for `instance_id`.
    ///
    /// The owner UID is derived from the Runtime state `root` directory, which
    /// the host creates owner-only before bootstrap. This matches the canonical
    /// `local:{instance}:uid:{uid}` LocalPrincipal derivation used by the
    /// Security owner at runtime, without any unsafe process-level FFI.
    pub fn first_init(instance_id: &InstanceId, root: &Path) -> Self {
        let owner_uid = owner_uid_of(root);
        let principal_ref = owner_principal_ref(instance_id, owner_uid);
        let issued_at = now_secs();
        Self {
            manifest_version: INSTANCE_MANIFEST_VERSION,
            instance_id: instance_id.clone(),
            host_generation: 1,
            data_schema_version: DEFAULT_DATA_SCHEMA_VERSION,
            owner: OwnerInitialization {
                principal_ref,
                owner_uid,
                authority_binding_id: format!("owner-binding-{}", instance_id.0),
                authority_generation: INITIAL_POLICY_GENERATION,
                issued_at,
            },
            policy_generations: DefaultPolicyGenerations::initial(),
        }
    }

    /// Structural correctness of a decoded manifest, independent of any on-disk
    /// permission or continuity checks.
    pub fn validate(&self) -> Result<(), Error> {
        if self.manifest_version != INSTANCE_MANIFEST_VERSION {
            return Err(Error::CorruptState(format!(
                "unsupported manifest version {}, expected {INSTANCE_MANIFEST_VERSION}",
                self.manifest_version
            )));
        }
        if self.instance_id.0.trim().is_empty() {
            return Err(Error::CorruptState(
                "manifest instance id is empty".to_owned(),
            ));
        }
        if self.host_generation != 1 {
            return Err(Error::CorruptState(format!(
                "first-init manifest host generation must be 1, found {}",
                self.host_generation
            )));
        }
        if self.data_schema_version == 0 {
            return Err(Error::CorruptState(
                "manifest data schema version must be non-zero".to_owned(),
            ));
        }
        self.owner.validate()?;
        self.policy_generations.validate()?;
        Ok(())
    }

    /// Atomically prepare the manifest as an owner-only file next to the
    /// eventual commit marker.
    ///
    /// The manifest is written to a unique private temp file, fsynced, and
    /// atomically hard-linked into place with no-replace semantics so that a
    /// concurrent candidate cannot clobber an already prepared manifest. The
    /// caller publishes `instance.id` only after this returns, giving
    /// commit-last visibility.
    pub fn prepare(&self, root: &Path) -> Result<(), Error> {
        self.validate()?;
        let dest = root.join(INSTANCE_MANIFEST_FILE);
        if dest.exists() {
            // A prior candidate (or attach) already prepared the manifest; it
            // must describe the same committed identity to be reusable.
            let existing = Self::load(root)?;
            if existing.instance_id != self.instance_id {
                return Err(Error::CorruptState(format!(
                    "manifest already exists for a different instance {}",
                    existing.instance_id.0
                )));
            }
            return Ok(());
        }

        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| Error::CorruptState(format!("serialize manifest: {error}")))?;
        if bytes.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(Error::CorruptState(
                "serialized manifest exceeds size bound".to_owned(),
            ));
        }

        let candidate = root.join(format!(
            ".{INSTANCE_MANIFEST_FILE}.{}.{}.tmp",
            std::process::id(),
            unique_suffix()
        ));
        let prepared = (|| -> Result<(), Error> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
                .map_err(io_err)?;
            harden_file(&candidate)?;
            file.write_all(&bytes).map_err(io_err)?;
            file.sync_all().map_err(io_err)?;
            drop(file);

            match fs::hard_link(&candidate, &dest) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    let existing = Self::load(root)?;
                    if existing.instance_id != self.instance_id {
                        return Err(Error::CorruptState(format!(
                            "manifest already exists for a different instance {}",
                            existing.instance_id.0
                        )));
                    }
                    Ok(())
                }
                Err(error) => Err(io_err(error)),
            }
        })();
        let _ = fs::remove_file(&candidate);
        prepared
    }

    /// Load and fully validate the manifest from `root`, failing closed on any
    /// corruption, wrong ownership, or unsafe file type.
    pub fn load(root: &Path) -> Result<Self, Error> {
        let path = root.join(INSTANCE_MANIFEST_FILE);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Err(Error::CorruptState(format!(
                    "instance manifest is missing: {}",
                    path.display()
                )));
            }
            Err(error) => return Err(io_err(error)),
        };
        validate_manifest_metadata(&path, &metadata)?;
        if metadata.len() > MAX_MANIFEST_BYTES {
            return Err(Error::CorruptState(format!(
                "instance manifest exceeds {MAX_MANIFEST_BYTES} bytes: {}",
                path.display()
            )));
        }

        let mut data = Vec::with_capacity(metadata.len() as usize);
        File::open(&path)
            .and_then(|file| {
                file.take(MAX_MANIFEST_BYTES + 1)
                    .read_to_end(&mut data)
                    .map(|_| ())
            })
            .map_err(io_err)?;
        if data.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(Error::CorruptState(
                "instance manifest grew beyond size bound during read".to_owned(),
            ));
        }

        let manifest: Self = serde_json::from_slice(&data)
            .map_err(|error| Error::CorruptState(format!("decode manifest: {error}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Load the manifest and assert it describes `expected` for restart identity
    /// continuity. A mismatch fails closed rather than adopting a foreign
    /// identity.
    pub fn load_for_instance(root: &Path, expected: &InstanceId) -> Result<Self, Error> {
        let manifest = Self::load(root)?;
        if manifest.instance_id != *expected {
            return Err(Error::CorruptState(format!(
                "manifest instance {} does not match committed instance {}",
                manifest.instance_id.0, expected.0
            )));
        }
        Ok(manifest)
    }
}

fn validate_manifest_metadata(path: &Path, metadata: &fs::Metadata) -> Result<(), Error> {
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(Error::CorruptState(format!(
            "instance manifest is not a direct regular file: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    {
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::CorruptState(format!(
                "instance manifest permissions are not owner-only: {}",
                path.display()
            )));
        }
        // The manifest must be owned by whoever owns its containing state root.
        if let Some(parent) = path.parent() {
            if let Some(owner_uid) = owner_uid_of(parent) {
                if metadata.uid() != owner_uid {
                    return Err(Error::CorruptState(format!(
                        "instance manifest owner does not match state root owner: {}",
                        path.display()
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Owner UID of `path`, or `None` off unix or when the path cannot be stat'd.
#[cfg(unix)]
fn owner_uid_of(path: &Path) -> Option<u32> {
    fs::symlink_metadata(path)
        .ok()
        .map(|metadata| metadata.uid())
}

#[cfg(not(unix))]
fn owner_uid_of(_path: &Path) -> Option<u32> {
    None
}

fn owner_principal_ref(instance_id: &InstanceId, owner_uid: Option<u32>) -> PrincipalRef {
    match owner_uid {
        Some(uid) => PrincipalRef(format!("local:{}:uid:{uid}", instance_id.0)),
        None => PrincipalRef(format!("local:{}:owner", instance_id.0)),
    }
}

#[cfg(unix)]
fn harden_file(path: &Path) -> Result<(), Error> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(io_err)
}

#[cfg(not(unix))]
fn harden_file(_path: &Path) -> Result<(), Error> {
    Ok(())
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn io_err(error: std::io::Error) -> Error {
    Error::Io(error.to_string())
}
