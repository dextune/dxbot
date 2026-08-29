//! Owner-only production Provider composition.
//!
//! Runtime Host owns config loading/composition; ProviderHost remains the
//! registration/lifecycle owner. Config stores only a credential reference.
//! Secret material is resolved ephemerally and is never serialized or logged.

#![forbid(unsafe_code)]

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use dxbot_core::types::ProviderId;
use provider_host::{DeepSeekFlashAdapter, HttpTransport, ProviderHost, ProviderStatus};
use serde::Deserialize;

pub(crate) const PROVIDER_CONFIG_FILE: &str = "provider-config.json";
const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const MIN_TIMEOUT_MS: u64 = 100;
const MAX_TIMEOUT_MS: u64 = 120_000;

#[derive(Debug)]
pub(crate) struct ProviderComposition {
    pub host: ProviderHost,
    pub provider_id: String,
    pub provider_generation: i64,
    pub provider_ready: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderConfig {
    schema_version: u32,
    adapter: String,
    provider_id: String,
    capability: String,
    generation: i64,
    endpoint: String,
    model: String,
    credential_ref: String,
    timeout_ms: u64,
}

pub(crate) fn load_provider_composition(
    runtime_root: &Path,
) -> Result<ProviderComposition, io::Error> {
    load_with_secret_resolver(runtime_root, |name| std::env::var(name).ok())
}

fn load_with_secret_resolver(
    runtime_root: &Path,
    resolve_secret: impl FnOnce(&str) -> Option<String>,
) -> Result<ProviderComposition, io::Error> {
    let path = runtime_root.join(PROVIDER_CONFIG_FILE);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ProviderComposition {
                host: ProviderHost::new(),
                provider_id: "unconfigured".to_owned(),
                provider_generation: 0,
                provider_ready: false,
            });
        }
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(invalid_config(
            "Provider config must be a direct regular file",
        ));
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Provider config must be owner-only (mode 0600)",
        ));
    }
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(invalid_config("Provider config exceeds 64 KiB"));
    }
    let bytes = fs::read(&path)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_CONFIG_BYTES {
        return Err(invalid_config("Provider config grew beyond 64 KiB"));
    }
    let config: ProviderConfig = serde_json::from_slice(&bytes)
        .map_err(|_| invalid_config("Provider config is not valid schema v1 JSON"))?;
    validate_config(&config)?;

    let secret_name = config
        .credential_ref
        .strip_prefix("env:")
        .ok_or_else(|| invalid_config("credential_ref must use env:<NAME>"))?;
    if !valid_secret_name(secret_name) {
        return Err(invalid_config(
            "credential_ref contains an invalid environment name",
        ));
    }
    let secret = resolve_secret(secret_name).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Provider credential reference is unresolved: env:{secret_name}"),
        )
    })?;
    if secret.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("Provider credential reference is empty: env:{secret_name}"),
        ));
    }

    let transport = HttpTransport::new(&config.endpoint, Duration::from_millis(config.timeout_ms))
        .with_bearer(&secret)
        .map_err(|_| invalid_config("Provider credential cannot be installed"))?;
    drop(secret);

    let mut host = ProviderHost::new();
    host.set_transport(transport);
    host.register_real_provider(Box::new(DeepSeekFlashAdapter::configured(
        ProviderId(config.provider_id.clone()),
        &config.capability,
        config.generation,
        &config.model,
    )))
    .map_err(|_| invalid_config("Provider registration is invalid"))?;
    let registered = host
        .list_providers(Some(&config.capability))
        .into_iter()
        .find(|provider| provider.id.0 == config.provider_id)
        .ok_or_else(|| invalid_config("Provider registration was not retained"))?;

    Ok(ProviderComposition {
        host,
        provider_id: registered.id.0,
        provider_generation: registered.generation,
        provider_ready: registered.status == ProviderStatus::Ready,
    })
}

fn validate_config(config: &ProviderConfig) -> Result<(), io::Error> {
    if config.schema_version != 1 {
        return Err(invalid_config("unsupported Provider config schema version"));
    }
    if config.adapter != "deepseek-flash" {
        return Err(invalid_config("unsupported production Provider adapter"));
    }
    for (name, value) in [
        ("provider_id", config.provider_id.as_str()),
        ("capability", config.capability.as_str()),
        ("model", config.model.as_str()),
    ] {
        if value.trim().is_empty() || value.len() > 256 {
            return Err(invalid_config(&format!(
                "invalid Provider config field: {name}"
            )));
        }
    }
    if config.generation <= 0 {
        return Err(invalid_config("Provider generation must be positive"));
    }
    if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&config.timeout_ms) {
        return Err(invalid_config(
            "Provider timeout_ms is outside the bounded range",
        ));
    }
    if config.endpoint.len() > 2048
        || !(config.endpoint.starts_with("http://") || config.endpoint.starts_with("https://"))
        || config
            .endpoint
            .split_once("://")
            .is_some_and(|(_, authority)| authority.contains('@'))
    {
        return Err(invalid_config(
            "Provider endpoint must be bounded HTTP(S) without embedded credentials",
        ));
    }
    Ok(())
}

fn valid_secret_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_uppercase() || (index > 0 && byte.is_ascii_digit())
        })
}

fn invalid_config(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempRoot(std::path::PathBuf);

    impl TempRoot {
        fn new() -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("dxb-provider-config-{suffix}"));
            fs::create_dir_all(&path).expect("create temp root");
            Self(path)
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_config(root: &Path, credential_ref: &str) {
        let path = root.join(PROVIDER_CONFIG_FILE);
        fs::write(
            &path,
            serde_json::json!({
                "schema_version": 1,
                "adapter": "deepseek-flash",
                "provider_id": "deepseek-production",
                "capability": "llm-chat",
                "generation": 7,
                "endpoint": "http://127.0.0.1:10000",
                "model": "test-model",
                "credential_ref": credential_ref,
                "timeout_ms": 30000
            })
            .to_string(),
        )
        .expect("write config");
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("permissions");
    }

    #[test]
    fn absent_config_is_explicitly_unconfigured_without_reference_fallback() {
        let root = TempRoot::new();
        let composition = load_with_secret_resolver(&root.0, |_| None).expect("unconfigured");
        assert_eq!(composition.provider_id, "unconfigured");
        assert!(!composition.provider_ready);
        assert!(composition.host.list_providers(None).is_empty());
    }

    #[test]
    fn configured_provider_uses_reference_not_secret_in_projection() {
        let root = TempRoot::new();
        write_config(&root.0, "env:DXBOT_TEST_PROVIDER_TOKEN");
        let secret = "secret-canary-must-not-appear";
        let composition = load_with_secret_resolver(&root.0, |name| {
            assert_eq!(name, "DXBOT_TEST_PROVIDER_TOKEN");
            Some(secret.to_owned())
        })
        .expect("configured");
        assert_eq!(composition.provider_id, "deepseek-production");
        assert!(composition.provider_ready);
        let rendered = format!("{composition:?}");
        assert!(!rendered.contains(secret));
        let providers = composition.host.list_providers(None);
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].generation, 7);
    }

    #[test]
    fn unresolved_credential_fails_closed_without_secret_material() {
        let root = TempRoot::new();
        write_config(&root.0, "env:DXBOT_MISSING_PROVIDER_TOKEN");
        let error = load_with_secret_resolver(&root.0, |_| None).expect_err("must fail");
        let rendered = error.to_string();
        assert!(rendered.contains("env:DXBOT_MISSING_PROVIDER_TOKEN"));
        assert!(!rendered.contains("Bearer"));
    }

    #[test]
    fn config_permissions_are_owner_only() {
        let root = TempRoot::new();
        write_config(&root.0, "env:DXBOT_TEST_PROVIDER_TOKEN");
        fs::set_permissions(
            root.0.join(PROVIDER_CONFIG_FILE),
            fs::Permissions::from_mode(0o644),
        )
        .expect("permissions");
        let error = load_with_secret_resolver(&root.0, |_| Some("secret".to_owned()))
            .expect_err("must reject broad permissions");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }
}
