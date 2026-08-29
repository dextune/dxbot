//! A3 provider adapter conformance and A4 sandbox canary evidence.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use provider_host::{
    CancellationToken, ConformanceError, ConformanceRequest, CredentialMaterial,
    HttpReferenceAdapter, HttpTransport, ReferenceAdapter, SubprocessReferenceAdapter, UsageSource,
};

fn granted_actions() -> BTreeSet<String> {
    BTreeSet::from(["tool.echo".to_owned()])
}

fn common_request<'a>(grants: &'a BTreeSet<String>) -> ConformanceRequest<'a> {
    ConformanceRequest {
        model: "fixture-model",
        prompt: "hello",
        deadline: Duration::from_secs(2),
        cancellation: CancellationToken::new(),
        credential: Some(CredentialMaterial::new(
            "credential://fixture",
            "fixture-secret",
        )),
        required_action_grant: Some("tool.echo"),
        action_grants: grants,
    }
}

fn assert_common_conformance(adapter: &dyn ReferenceAdapter) {
    let grants = granted_actions();
    let outcome = adapter
        .execute(&common_request(&grants))
        .expect("reference adapter conformance");
    assert_eq!(outcome.content, "hello");
    assert_eq!(outcome.usage.input_units, Some(3));
    assert_eq!(outcome.usage.output_units, Some(5));
    assert_eq!(outcome.usage.source, UsageSource::Reported);
}

#[test]
fn http_reference_adapter_passes_common_conformance() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture server");
    let address = listener.local_addr().expect("fixture address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept fixture connection");
        let mut request = [0_u8; 8192];
        let read = stream.read(&mut request).expect("read fixture request");
        let request = String::from_utf8_lossy(&request[..read]).to_lowercase();
        assert!(request.contains("authorization: bearer fixture-secret"));

        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":5}}\n\n"
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write fixture response");
    });

    let transport = HttpTransport::new(&format!("http://{address}"), Duration::from_secs(2));
    let adapter = HttpReferenceAdapter::new(transport).expect("build http reference adapter");
    assert_common_conformance(&adapter);
    server.join().expect("fixture server join");
}

#[cfg(unix)]
#[test]
fn subprocess_reference_adapter_passes_same_common_conformance() {
    let root = temp_root("subprocess-conformance");
    let script = write_script(
        &root,
        "provider-fixture.sh",
        concat!(
            "cat >/dev/null\n",
            "[ \"$DXBOT_REFERENCE_CREDENTIAL\" = \"fixture-secret\" ] || exit 8\n",
            "printf 'text\\thello\\n'\n",
            "printf 'tool\\ttool.echo\\n'\n",
            "printf 'usage\\t3\\t5\\treported\\n'\n",
            "printf 'done\\n'\n"
        ),
    );
    let adapter = SubprocessReferenceAdapter::new(&script, &[]);
    assert_common_conformance(&adapter);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn subprocess_tool_request_cannot_bypass_action_grant() {
    let root = temp_root("subprocess-grant");
    let script = write_script(
        &root,
        "provider-tool.sh",
        "cat >/dev/null\nprintf 'tool\\ttool.echo\\n'\nprintf 'done\\n'\n",
    );
    let adapter = SubprocessReferenceAdapter::new(&script, &[]);
    let grants = BTreeSet::new();
    let request = ConformanceRequest {
        model: "fixture-model",
        prompt: "hello",
        deadline: Duration::from_secs(2),
        cancellation: CancellationToken::new(),
        credential: None,
        required_action_grant: None,
        action_grants: &grants,
    };
    assert_eq!(
        adapter.execute(&request),
        Err(ConformanceError::ActionGrantRequired {
            action: "tool.echo".to_owned(),
        })
    );
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn subprocess_cancellation_kills_process_before_publish() {
    let root = temp_root("subprocess-cancel");
    // The child runs far longer than any plausible scheduling delay so the only
    // way the adapter returns without cancellation is the deadline. The deadline
    // is set generously above worst-case canceller wakeup latency under parallel
    // test load, so cancellation deterministically wins over both completion and
    // the deadline.
    let script = write_script(
        &root,
        "provider-slow.sh",
        "cat >/dev/null\nsleep 30\nprintf 'text\\tlate\\n'\nprintf 'done\\n'\n",
    );
    let adapter = SubprocessReferenceAdapter::new(&script, &[]);
    let grants = BTreeSet::new();
    let cancellation = CancellationToken::new();
    let trigger = cancellation.clone();
    let canceller = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        trigger.cancel();
    });
    let request = ConformanceRequest {
        model: "fixture-model",
        prompt: "hello",
        deadline: Duration::from_secs(10),
        cancellation,
        credential: None,
        required_action_grant: None,
        action_grants: &grants,
    };
    assert_eq!(adapter.execute(&request), Err(ConformanceError::Cancelled));
    canceller.join().expect("cancel fixture join");
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn malformed_subprocess_frame_is_not_success() {
    let root = temp_root("subprocess-malformed");
    let script = write_script(
        &root,
        "provider-malformed.sh",
        "cat >/dev/null\nprintf 'not-a-frame\\n'\n",
    );
    let adapter = SubprocessReferenceAdapter::new(&script, &[]);
    let grants = BTreeSet::new();
    let request = ConformanceRequest {
        model: "fixture-model",
        prompt: "hello",
        deadline: Duration::from_secs(1),
        cancellation: CancellationToken::new(),
        credential: None,
        required_action_grant: None,
        action_grants: &grants,
    };
    assert_eq!(
        adapter.execute(&request),
        Err(ConformanceError::MalformedFrame)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn expired_credential_fails_before_transport_or_process_start() {
    let grants = BTreeSet::new();
    let request = ConformanceRequest {
        model: "fixture-model",
        prompt: "hello",
        deadline: Duration::from_secs(1),
        cancellation: CancellationToken::new(),
        credential: Some(
            CredentialMaterial::new("credential://expired", "never-log-this")
                .with_expiry(Instant::now()),
        ),
        required_action_grant: None,
        action_grants: &grants,
    };
    let transport = HttpTransport::new("http://127.0.0.1:9", Duration::from_millis(50));
    let adapter = HttpReferenceAdapter::new(transport).expect("http reference adapter");
    assert_eq!(
        adapter.execute(&request),
        Err(ConformanceError::CredentialExpired)
    );
}

#[cfg(unix)]
#[test]
fn provider_canary_runs_through_local_sandbox_lifecycle() {
    use runtime_security::{
        LocalSubprocessSandbox, NetworkPolicy, RuntimeArtifact, SandboxSpec, SandboxTerminal,
    };

    let artifact_root = temp_root("sandbox-provider-artifact");
    let workspace_root = temp_root("sandbox-provider-workspace");
    let bytes = b"#!/bin/sh\n[ \"$DXBOT_SANDBOX_NETWORK\" = \"deny\" ] || exit 9\nexit 0\n";
    let artifact =
        RuntimeArtifact::publish(&artifact_root, bytes).expect("publish provider canary");
    let spec = SandboxSpec {
        owner_instance_id: "instance-1".to_owned(),
        execution_id: "provider-canary".to_owned(),
        host_generation: 4,
        runtime_artifact: artifact,
        network_policy: NetworkPolicy::Deny,
        mounts: Vec::new(),
        execution_deadline: Duration::from_secs(1),
    };
    let sandbox = LocalSubprocessSandbox::new("instance-1", 4, &workspace_root);
    let cancelled = AtomicBool::new(false);
    let result = sandbox
        .run_with_control(&spec, &[], &cancelled)
        .expect("run sandbox provider canary");
    assert_eq!(result.terminal, SandboxTerminal::Completed);
    assert_eq!(result.exit_code, Some(0));
    assert!(!workspace_root.join("provider-canary").exists());
    let _ = std::fs::remove_dir_all(artifact_root);
    let _ = std::fs::remove_dir_all(workspace_root);
}

#[cfg(unix)]
fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("dxbot-{label}-{nanos}"))
}

#[cfg(unix)]
fn write_script(root: &Path, name: &str, body: &str) -> PathBuf {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    std::fs::create_dir_all(root).expect("create fixture root");
    let path = root.join(name);
    // Publish the executable durably before returning: write and fsync a
    // private temp file with the executable mode already set, close it, then
    // atomically rename it into place. Spawning an executable that this process
    // still holds open for writing can fail with `ETXTBSY` under parallel load;
    // renaming a fully-closed inode avoids that race entirely.
    let tmp = root.join(format!(".{name}.tmp"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o700)
        .open(&tmp)
        .expect("create fixture temp");
    file.write_all(format!("#!/bin/sh\n{body}").as_bytes())
        .expect("write fixture script");
    file.sync_all().expect("sync fixture script");
    drop(file);
    std::fs::rename(&tmp, &path).expect("publish fixture script");
    path
}
