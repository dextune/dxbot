//! Acceptance coverage for `AT-HARNESS-001`: Reference Provider plus one real
//! Harness Adapter canary.
#![allow(clippy::unwrap_used)]

use dxbot_core::types::ProviderId;
use provider_host::{
    HarnessAdapter, HarnessError, ProviderHost, ProviderStatus, ReferenceProvider, TaskDescription,
    TaskStatus,
};

/// Absolute wall-clock deadline (Unix epoch seconds) far enough in the future
/// that the bounded task never expires while the suite runs. `deadline` is an
/// absolute timestamp, not a relative budget, so a fixed past constant would
/// spuriously trip `DeadlineExceeded`.
fn future_deadline() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    i64::try_from(now).unwrap_or(i64::MAX).saturating_add(3_600)
}

fn task(intent: &str) -> TaskDescription {
    TaskDescription {
        intent: intent.to_string(),
        context: "bounded-context".to_string(),
        budget: Some(10),
        deadline: Some(future_deadline()),
    }
}

#[test]
fn harness_reference_provider_is_always_available() {
    let mut host = ProviderHost::new();
    host.register_reference_provider(ReferenceProvider::new(
        ProviderId("ref-1".to_string()),
        "text",
        1,
    ))
    .unwrap();

    // The Reference Provider is always available once registered and executes
    // the bounded task even with no real adapter.
    let result = host.execute_task(&task("write a plan")).unwrap();
    assert_eq!(result.output, "write a plan");
    assert_eq!(result.status, TaskStatus::Completed);
    assert!(
        result
            .evidence
            .iter()
            .any(|e| e.provider == ProviderId("ref-1".to_string()))
    );
}

#[test]
fn harness_real_adapter_canary_executes_task() {
    let mut host = ProviderHost::new();
    host.register_reference_provider(ReferenceProvider::new(
        ProviderId("ref-1".to_string()),
        "text",
        1,
    ))
    .unwrap();
    host.register_harness_adapter(HarnessAdapter::new(
        ProviderId("adapter-1".to_string()),
        "text",
        2,
    ))
    .unwrap();

    // The real adapter canary is preferred and actually executes the task
    // through the provider chain (its output marker differs from the fallback).
    let result = host.execute_task(&task("write a plan")).unwrap();
    assert_eq!(result.output, "adapter:write a plan");
    assert_eq!(result.status, TaskStatus::Completed);
    assert!(
        result
            .evidence
            .iter()
            .any(|e| e.provider == ProviderId("adapter-1".to_string())
                && e.observation == "harness-adapter-canary")
    );
}

#[test]
fn harness_provider_chain_falls_back_to_reference() {
    let mut host = ProviderHost::new();
    host.register_reference_provider(ReferenceProvider::new(
        ProviderId("ref-1".to_string()),
        "text",
        1,
    ))
    .unwrap();
    host.register_harness_adapter(HarnessAdapter::unavailable(
        ProviderId("adapter-1".to_string()),
        "text",
        2,
    ))
    .unwrap();

    // The real canary is unavailable, so the chain falls back to the always
    // available Reference Provider instead of failing the task.
    let result = host.execute_task(&task("write a plan")).unwrap();
    assert_eq!(result.output, "write a plan");
    assert!(
        result
            .evidence
            .iter()
            .any(|e| e.provider == ProviderId("ref-1".to_string()))
    );
}

#[test]
fn harness_list_providers_filters_by_capability() {
    let mut host = ProviderHost::new();
    host.register_reference_provider(ReferenceProvider::new(
        ProviderId("ref-text".to_string()),
        "text",
        1,
    ))
    .unwrap();
    host.register_harness_adapter(HarnessAdapter::new(
        ProviderId("adapter-code".to_string()),
        "code",
        2,
    ))
    .unwrap();

    let all = host.list_providers(None);
    assert_eq!(all.len(), 2);

    let text = host.list_providers(Some("text"));
    assert_eq!(text.len(), 1);
    assert_eq!(text[0].id, ProviderId("ref-text".to_string()));

    let code = host.list_providers(Some("code"));
    assert_eq!(code.len(), 1);
    assert_eq!(code[0].id, ProviderId("adapter-code".to_string()));

    let none = host.list_providers(Some("vision"));
    assert!(none.is_empty());

    let info = host
        .get_provider(&ProviderId("adapter-code".to_string()))
        .unwrap();
    assert_eq!(info.capability, "code");
    assert_eq!(info.status, ProviderStatus::Ready);
    assert!(matches!(
        host.get_provider(&ProviderId("missing".to_string())),
        Err(HarnessError::ProviderNotFound { .. })
    ));
}

#[cfg(unix)]
fn real_host(endpoint: &str) -> ProviderHost {
    let mut host = ProviderHost::new();
    host.set_transport(provider_host::HttpTransport::new(
        endpoint,
        std::time::Duration::from_secs(2),
    ));
    host.register_real_provider(Box::new(provider_host::DeepSeekFlashAdapter::configured(
        ProviderId("real-1".to_owned()),
        "llm-chat",
        1,
        "test-model",
    )))
    .unwrap();
    host
}

#[cfg(unix)]
fn execute_against_response(status: &str, headers: &str, body: String) -> HarnessError {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let status = status.to_owned();
    let headers = headers.to_owned();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).unwrap();
        write!(
            stream,
            "HTTP/1.1 {status}\r\ncontent-length: {}\r\n{headers}\r\n{}",
            body.len(),
            body
        )
        .unwrap();
        stream.flush().unwrap();
    });
    let error = match real_host(&endpoint).execute_task(&task("typed failure")) {
        Err(error) => error,
        Ok(result) => panic!("fixture unexpectedly succeeded: {result:?}"),
    };
    server.join().unwrap();
    error
}

#[cfg(unix)]
#[test]
fn real_provider_preserves_typed_failure_matrix_without_detail_leak() {
    assert_eq!(
        execute_against_response(
            "429 Too Many Requests",
            "retry-after: 7\r\n",
            "credential=secret-upstream".to_owned(),
        ),
        HarnessError::RateLimited {
            id: ProviderId("real-1".to_owned()),
            retry_after_seconds: Some(7),
        }
    );
    assert_eq!(
        execute_against_response(
            "503 Service Unavailable",
            "",
            "credential=secret-upstream".to_owned(),
        ),
        HarnessError::UpstreamUnavailable {
            id: ProviderId("real-1".to_owned()),
            status: 503,
        }
    );
    assert_eq!(
        execute_against_response(
            "200 OK",
            "content-type: application/json\r\n",
            "not-json".to_owned()
        ),
        HarnessError::ProtocolViolation {
            id: ProviderId("real-1".to_owned()),
        }
    );

    let oversized = serde_json::json!({
        "choices": [{"message": {"content": "x".repeat(1024 * 1024 + 1), "reasoning_content": null}}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "reasoning_tokens": 0}
    })
    .to_string();
    assert_eq!(
        execute_against_response("200 OK", "content-type: application/json\r\n", oversized),
        HarnessError::OutputExceeded {
            id: ProviderId("real-1".to_owned()),
        }
    );
}

#[cfg(unix)]
#[test]
fn real_provider_transport_and_deadline_are_typed_without_fallback_masking() {
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    assert_eq!(
        real_host(&endpoint).execute_task(&task("transport")),
        Err(HarnessError::TransportUnavailable {
            id: ProviderId("real-1".to_owned()),
        })
    );

    let expired = TaskDescription {
        intent: "deadline".to_owned(),
        context: "bounded".to_owned(),
        budget: Some(1),
        deadline: Some(1),
    };
    assert_eq!(
        real_host("http://127.0.0.1:9").execute_task(&expired),
        Err(HarnessError::DeadlineExceeded)
    );
}
