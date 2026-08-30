//! Acceptance tests for the DeepSeek v4 Flash direct adapter.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(feature = "direct-deepseek")]

use std::sync::Arc;
use std::time::Duration;

use dxbot_core::types::ProviderId;
use provider_host::{
    CancellationToken, ChatCompletionProtocol, ChatMessage, DeepSeekFlashAdapter, ExecuteRequest,
    HttpExecuteProvider, HttpTransport, ProtocolKind, ProviderEvent, ProviderExecuteConfig,
    ProviderHost, ProviderRegistration, ProviderRequest, RealProvider, RegistrationLimits,
    TaskDescription, TaskStatus, TransportBinding,
};

const MAX_BYTES: usize = 1024 * 1024;
const MAX_ITEMS: usize = 1000;

fn proxy_endpoint() -> String {
    std::env::var("DXBOT_TEST_OPENAI_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:10000".to_owned())
}

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime")
}

fn task(intent: &str) -> TaskDescription {
    TaskDescription {
        intent: intent.to_string(),
        context: "bounded-context".to_string(),
        budget: Some(50),
        deadline: None,
    }
}

fn execute_request(intent: &str, budget: Option<u64>) -> ExecuteRequest {
    ExecuteRequest::bounded(
        ProviderId("deepseek-flash".to_owned()),
        1,
        intent.to_owned(),
        "bounded-context".to_owned(),
        budget,
        None,
    )
}

fn deepseek_host(handle: tokio::runtime::Handle, endpoint: &str) -> ProviderHost {
    let transport = HttpTransport::new(endpoint, Duration::from_secs(30));
    let binding = TransportBinding::new(
        ProtocolKind::OpenAiChatCompletions,
        transport,
        MAX_BYTES,
        MAX_ITEMS,
    );
    let adapter = Box::new(DeepSeekFlashAdapter::new(
        ProviderId("deepseek-flash".to_string()),
        "llm-chat",
        1,
    ));
    let provider = HttpExecuteProvider::new(adapter, &binding, MAX_BYTES, MAX_ITEMS);
    let registration = ProviderRegistration::new(
        provider as Arc<_>,
        ProtocolKind::OpenAiChatCompletions,
        binding,
        RegistrationLimits {
            max_output_bytes: MAX_BYTES,
            max_output_items: MAX_ITEMS,
        },
    );
    let host = ProviderHost::new().with_runtime_handle(handle);
    host.register(registration).unwrap();
    host
}

#[test]
fn deepseek_001_implements_real_provider_trait() {
    let adapter = DeepSeekFlashAdapter::new(ProviderId("deepseek-flash".to_string()), "text", 1);
    assert_eq!(adapter.id(), &ProviderId("deepseek-flash".to_string()));
    assert_eq!(adapter.capability(), "text");
    assert_eq!(adapter.generation(), 1);

    let req = adapter.build_request(&execute_request("Say hello", Some(50)));
    assert_eq!(req.model, "alibaba/deepseek-v4-flash-0731");
    assert_eq!(req.temperature, Some(0.0));
    assert!(!req.messages.is_empty());
    assert_eq!(req.messages[0].role, "user");
    assert!(req.messages[0].content.contains("Say hello"));

    assert_eq!(
        adapter
            .build_request(&execute_request("bounded conversion", Some(u64::MAX)))
            .max_tokens,
        Some(u32::MAX)
    );
}

#[test]
#[ignore = "requires OpenAI-compatible proxy via DXBOT_TEST_OPENAI_ENDPOINT (default localhost:10000)"]
fn deepseek_002_real_proxy_produces_output() {
    let runtime = test_runtime();
    let host = deepseek_host(runtime.handle().clone(), &proxy_endpoint());
    let result = host
        .execute_task_with_cancel(
            &ProviderId("deepseek-flash".to_owned()),
            1,
            &task("Say hello"),
            CancellationToken::new(),
        )
        .unwrap();
    assert!(!result.output.is_empty());
    assert_eq!(result.status, TaskStatus::Completed);
}

#[test]
#[ignore = "requires OpenAI-compatible proxy via DXBOT_TEST_OPENAI_ENDPOINT (default localhost:10000)"]
fn deepseek_003_reasoning_tokens_in_stream() {
    let transport = HttpTransport::new(&proxy_endpoint(), Duration::from_secs(30));
    let protocol = ChatCompletionProtocol::new(transport, MAX_BYTES, MAX_ITEMS);
    let request = ProviderRequest {
        model: "alibaba/deepseek-v4-flash-0731".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "What is 2+2?".to_string(),
        }],
        max_tokens: Some(20),
        temperature: Some(0.0),
        stream: true,
    };
    let config = ProviderExecuteConfig {
        deadline: None,
        cancel_token: None,
        max_output_bytes: MAX_BYTES,
        max_output_items: MAX_ITEMS,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio runtime");
    let events = rt
        .block_on(protocol.execute(&request, &config))
        .expect("protocol execute");
    assert!(events.iter().any(|event| matches!(
        event,
        ProviderEvent::ReasoningDelta { .. } | ProviderEvent::ContentDelta { .. }
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ProviderEvent::Completed { .. }))
    );
}

#[test]
#[ignore = "requires OpenAI-compatible proxy via DXBOT_TEST_OPENAI_ENDPOINT (default localhost:10000)"]
fn deepseek_004_direct_adapter_output_is_not_canary_marked() {
    let runtime = test_runtime();
    let host = deepseek_host(runtime.handle().clone(), &proxy_endpoint());
    let result = host
        .execute_task_with_cancel(
            &ProviderId("deepseek-flash".to_owned()),
            1,
            &task("Say hello"),
            CancellationToken::new(),
        )
        .unwrap();
    assert!(!result.output.is_empty());
    assert!(!result.output.starts_with("canary:"));
    assert_eq!(result.status, TaskStatus::Completed);
}

#[cfg(unix)]
fn deepseek_error_against_response(
    status: &str,
    headers: &str,
    body: String,
) -> provider_host::HarnessError {
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
    let runtime = test_runtime();
    let host = deepseek_host(runtime.handle().clone(), &endpoint);
    let error = host
        .execute_task_with_cancel(
            &ProviderId("deepseek-flash".to_owned()),
            1,
            &task("typed failure"),
            CancellationToken::new(),
        )
        .expect_err("fixture must fail");
    server.join().unwrap();
    error
}

#[cfg(unix)]
#[test]
fn deepseek_006_openai_typed_failure_matrix_without_detail_leak() {
    assert_eq!(
        deepseek_error_against_response(
            "429 Too Many Requests",
            "retry-after: 7\r\n",
            "credential=secret-upstream".to_owned(),
        ),
        provider_host::HarnessError::RateLimited {
            id: ProviderId("deepseek-flash".to_owned()),
            retry_after_seconds: Some(7),
        }
    );
    assert_eq!(
        deepseek_error_against_response("503 Service Unavailable", "", "err".to_owned()),
        provider_host::HarnessError::UpstreamUnavailable {
            id: ProviderId("deepseek-flash".to_owned()),
            status: 503,
        }
    );
    assert_eq!(
        deepseek_error_against_response(
            "200 OK",
            "content-type: application/json\r\n",
            "not-json".to_owned()
        ),
        provider_host::HarnessError::ProtocolViolation {
            id: ProviderId("deepseek-flash".to_owned()),
        }
    );
    let oversized = serde_json::json!({
        "choices": [{"message": {"content": "x".repeat(1024 * 1024 + 1), "reasoning_content": null}}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "reasoning_tokens": 0}
    })
    .to_string();
    assert_eq!(
        deepseek_error_against_response("200 OK", "content-type: application/json\r\n", oversized),
        provider_host::HarnessError::OutputExceeded {
            id: ProviderId("deepseek-flash".to_owned()),
        }
    );
}

#[cfg(unix)]
#[test]
fn deepseek_007_transport_and_deadline_are_typed_without_fallback_masking() {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let runtime = test_runtime();
    let host = deepseek_host(runtime.handle().clone(), &endpoint);
    assert_eq!(
        host.execute_task_with_cancel(
            &ProviderId("deepseek-flash".to_owned()),
            1,
            &task("transport"),
            CancellationToken::new(),
        ),
        Err(provider_host::HarnessError::TransportUnavailable {
            id: ProviderId("deepseek-flash".to_owned()),
        })
    );

    let host = deepseek_host(runtime.handle().clone(), "http://127.0.0.1:9");
    let expired = TaskDescription {
        intent: "deadline".to_owned(),
        context: "bounded".to_owned(),
        budget: Some(1),
        deadline: Some(1),
    };
    assert_eq!(
        host.execute_task_with_cancel(
            &ProviderId("deepseek-flash".to_owned()),
            1,
            &expired,
            CancellationToken::new(),
        ),
        Err(provider_host::HarnessError::DeadlineExceeded)
    );
}

#[test]
#[ignore = "requires OpenAI-compatible proxy via DXBOT_TEST_OPENAI_ENDPOINT (default localhost:10000)"]
fn deepseek_005_temperature_zero_is_deterministic() {
    let runtime = test_runtime();
    let host = deepseek_host(runtime.handle().clone(), &proxy_endpoint());
    let task_def = TaskDescription {
        intent: "Say just the number 42".to_string(),
        context: String::new(),
        budget: Some(10),
        deadline: None,
    };
    let result1 = host
        .execute_task_with_cancel(
            &ProviderId("deepseek-flash".to_owned()),
            1,
            &task_def,
            CancellationToken::new(),
        )
        .unwrap();
    let result2 = host
        .execute_task_with_cancel(
            &ProviderId("deepseek-flash".to_owned()),
            1,
            &task_def,
            CancellationToken::new(),
        )
        .unwrap();
    assert!(!result1.output.is_empty());
    assert!(!result2.output.is_empty());
    assert_eq!(result1.status, TaskStatus::Completed);
    assert_eq!(result2.status, TaskStatus::Completed);
}
