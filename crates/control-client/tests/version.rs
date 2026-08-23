//! Acceptance tests for `AT-VERSION-001`: minimum client/protocol/schema
//! compatibility query on the control client.
//!
//! `cargo test -p control-client version` runs every test below. The version
//! compatibility matrix is injected in-memory via the builder, so no network
//! I/O is required.

use control_client::SubmissionClient;
use dxbot_core::types::{InstanceId, RemoteVersionInfo, VersionInfo};

/// A version matrix fixture mixing a compatible remote handshake verdict with
/// the minimum supported protocol/schema major.minor versions.
fn sample_version() -> VersionInfo {
    VersionInfo {
        client_version: "0.1.0".to_owned(),
        supported_protocol_versions: vec!["1.0".to_owned()],
        supported_schema_versions: vec!["1.0".to_owned()],
        remote_compatibility: Some(RemoteVersionInfo {
            runtime_version: "0.1.0".to_owned(),
            protocol_version: "1.0".to_owned(),
            schema_version: "1.0".to_owned(),
            compatible: true,
        }),
    }
}

/// A client advertising the fixture version matrix.
fn client_with_version(version: VersionInfo) -> SubmissionClient {
    SubmissionClient::builder(InstanceId("instance-1".to_owned()))
        .with_version(version)
        .build()
}

#[test]
fn version_compatibility_query_returns_version_info() {
    let version = sample_version();
    let client = client_with_version(version.clone());

    let queried = client.get_version_compatibility().unwrap();

    // The query returns the injected matrix wholesale.
    assert_eq!(queried, version);
}

#[test]
fn version_matrix_includes_minimum_protocol_and_schema() {
    let client = client_with_version(sample_version());

    let info = client.get_version_compatibility().unwrap();

    // The matrix carries the minimum supported protocol and schema versions.
    assert!(
        info.supported_protocol_versions
            .iter()
            .any(|v| v == "1.0"),
        "protocol v1.0 must be advertised in the matrix"
    );
    assert!(
        info.supported_schema_versions.iter().any(|v| v == "1.0"),
        "schema v1.0 must be advertised in the matrix"
    );
}

#[test]
fn version_remote_compatibility_reports_compatible_when_match() {
    let client = client_with_version(sample_version());

    let info = client.get_version_compatibility().unwrap();

    // A remote that matches the advertised protocol/schema version is reported
    // as compatible.
    let remote = info
        .remote_compatibility
        .expect("remote handshake must be present in the fixture matrix");
    assert!(remote.compatible);
    assert_eq!(remote.protocol_version, "1.0");
    assert_eq!(remote.schema_version, "1.0");
}