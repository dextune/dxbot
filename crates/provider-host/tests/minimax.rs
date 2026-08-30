//! MiniMax M3 Anthropic-compatible adapter acceptance tests.
//!
//! Preserves the direct MiniMax execution evidence under the canonical async
//! Execute contract and per-registration protocol/transport binding.
#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "direct-minimax")]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use dxbot_core::types::ProviderId;
use provider_host::{
    ExecuteRequest, HttpExecuteProvider, HttpTransport, MiniMaxM3Adapter, ProtocolKind,
    ProviderHost, ProviderRegistration, RealProvider, RegistrationLimits, TaskDescription,
    TaskStatus, TransportBinding,
};

const MAX_BYTES: usize = 1024 * 1024;
const MAX_ITEMS: usize = 1000;

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime")
}

fn task(intent: &str) -> TaskDescription {
    TaskDescription {
        intent: intent.to_owned(),
        context: "bounded-context".to_owned(),
        budget: Some(32),
        deadline: None,
    }
}

fn minimax_host(handle: tokio::runtime::Handle, transport: HttpTransport) -> ProviderHost {
    let binding = TransportBinding::new(
        ProtocolKind::AnthropicMessages,
        transport,
        MAX_BYTES,
        MAX_ITEMS,
    );
    let adapter = Box::new(MiniMaxM3Adapter::new(
        ProviderId("minimax-m3".to_owned()),
        "llm-chat",
        1,
    ));
    let provider = HttpExecuteProvider::new(adapter, &binding, MAX_BYTES, MAX_ITEMS);
    let registration = ProviderRegistration::new(
        provider as Arc<_>,
        ProtocolKind::AnthropicMessages,
        binding,
        RegistrationLimits {
            max_output_bytes: MAX_BYTES,
            max_output_items: MAX_ITEMS,
        },
    );
    let host = ProviderHost::new().with_runtime_handle(handle);
    host.register(registration).expect("register MiniMax");
    host
}

#[test]
fn minimax_m3_adapter_builds_bounded_non_stream_request() {
    let adapter = MiniMaxM3Adapter::new(ProviderId("minimax-m3".to_owned()), "llm-chat", 1);
    let request = ExecuteRequest::bounded(
        ProviderId("minimax-m3".to_owned()),
        1,
        "reply briefly".to_owned(),
        "bounded-context".to_owned(),
        Some(32),
        None,
    );
    let built = adapter.build_request(&request);
    assert_eq!(built.model, "MiniMax-M3");
    assert_eq!(built.max_tokens, Some(32));
    assert_eq!(built.temperature, Some(0.0));
    assert!(!built.stream);
    assert_eq!(built.messages.len(), 1);
    assert!(built.messages[0].content.contains("reply briefly"));
    assert!(built.messages[0].content.contains("bounded-context"));
}

#[cfg(unix)]
#[test]
fn minimax_anthropic_messages_executes_through_common_host() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let endpoint = format!("http://{}/anthropic/v1", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept fixture");
        let mut request = vec![0_u8; 16 * 1024];
        let size = stream.read(&mut request).expect("read request");
        request.truncate(size);
        let text = String::from_utf8_lossy(&request);
        let lower = text.to_ascii_lowercase();
        assert!(text.starts_with("POST /anthropic/v1/messages HTTP/1.1"));
        assert!(lower.contains("x-api-key: fixture-secret"));
        assert!(lower.contains("anthropic-version: 2023-06-01"));
        assert!(text.contains("MiniMax-M3"));
        assert!(text.contains("DXBOT_MINIMAX_FIXTURE"));

        let body = serde_json::json!({
            "id": "msg_fixture",
            "type": "message",
            "role": "assistant",
            "model": "MiniMax-M3",
            "content": [{"type": "text", "text": "DXBOT_MINIMAX_FIXTURE_OK"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 12, "output_tokens": 5}
        })
        .to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write response");
        stream.flush().expect("flush response");
    });

    let transport = HttpTransport::new(&endpoint, Duration::from_secs(2))
        .with_anthropic_key("fixture-secret")
        .expect("install fixture credential");
    assert!(!format!("{transport:?}").contains("fixture-secret"));
    let runtime = test_runtime();
    let host = minimax_host(runtime.handle().clone(), transport);

    let result = host
        .execute_task_with_cancel(
            &ProviderId("minimax-m3".to_owned()),
            1,
            &task("DXBOT_MINIMAX_FIXTURE"),
            provider_host::CancellationToken::new(),
        )
        .expect("execute MiniMax fixture");
    server.join().expect("fixture server");
    assert_eq!(result.status, TaskStatus::Completed);
    assert_eq!(result.output, "DXBOT_MINIMAX_FIXTURE_OK");
    assert_eq!(result.evidence.len(), 1);
    assert_eq!(
        result.evidence[0].provider,
        ProviderId("minimax-m3".to_owned())
    );
}

#[cfg(unix)]
fn minimax_error_against_response(status: &str, body: String) -> provider_host::HarnessError {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind error fixture");
    let endpoint = format!("http://{}/anthropic/v1", listener.local_addr().unwrap());
    let status = status.to_owned();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept error fixture");
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).expect("read error request");
        write!(
            stream,
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write error response");
        stream.flush().expect("flush error response");
    });

    let transport = HttpTransport::new(&endpoint, Duration::from_secs(2))
        .with_anthropic_key("error-secret")
        .expect("install error credential");
    let runtime = test_runtime();
    let host = minimax_host(runtime.handle().clone(), transport);
    let error = host
        .execute_task_with_cancel(
            &ProviderId("minimax-m3".to_owned()),
            1,
            &task("typed failure"),
            provider_host::CancellationToken::new(),
        )
        .expect_err("fixture must fail");
    server.join().expect("error fixture server");
    error
}

#[cfg(unix)]
#[test]
fn minimax_anthropic_failures_are_typed_and_bounded() {
    assert_eq!(
        minimax_error_against_response(
            "401 Unauthorized",
            r#"{"type":"error","error":{"message":"credential=secret-upstream"}}"#.to_owned(),
        ),
        provider_host::HarnessError::InvalidRequest {
            id: ProviderId("minimax-m3".to_owned()),
            status: 401,
        }
    );
    assert_eq!(
        minimax_error_against_response("200 OK", "not-json".to_owned()),
        provider_host::HarnessError::ProtocolViolation {
            id: ProviderId("minimax-m3".to_owned()),
        }
    );
    let oversized = serde_json::json!({
        "content": [{"type": "text", "text": "x".repeat(1024 * 1024 + 1)}],
        "usage": {"input_tokens": 1, "output_tokens": 1}
    })
    .to_string();
    assert_eq!(
        minimax_error_against_response("200 OK", oversized),
        provider_host::HarnessError::OutputExceeded {
            id: ProviderId("minimax-m3".to_owned()),
        }
    );
}

#[test]
#[ignore = "requires MINIMAX_API_KEY and optional DXBOT_TEST_MINIMAX_ENDPOINT"]
fn minimax_m3_live_canary_returns_non_empty_output() {
    let key = std::env::var("MINIMAX_API_KEY").expect("MINIMAX_API_KEY");
    let endpoint = std::env::var("DXBOT_TEST_MINIMAX_ENDPOINT")
        .unwrap_or_else(|_| "https://api.minimax.io/anthropic/v1".to_owned());
    let transport = HttpTransport::new(&endpoint, Duration::from_secs(60))
        .with_anthropic_key(&key)
        .expect("install MiniMax credential");
    drop(key);

    let runtime = test_runtime();
    let host = minimax_host(runtime.handle().clone(), transport);
    let result = host
        .execute_task_with_cancel(
            &ProviderId("minimax-m3".to_owned()),
            1,
            &task("Reply with exactly DXBOT_MINIMAX_OK and nothing else."),
            provider_host::CancellationToken::new(),
        )
        .expect("live MiniMax response");
    assert_eq!(result.status, TaskStatus::Completed);
    assert!(!result.output.trim().is_empty());
}
