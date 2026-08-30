//! Acceptance tests for provider common infrastructure (AT-PROVIDER-INFRA-001..008).

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(feature = "direct-deepseek")]

use dxbot_core::types::ProviderId;
use provider_host::{
    CancellationToken, ChatCompletionProtocol, ChatMessage, DeepSeekFlashAdapter,
    HttpExecuteProvider, HttpTransport, ProtocolKind, ProviderError, ProviderEvent,
    ProviderExecuteConfig, ProviderHost, ProviderRegistration, ProviderRequest, RegistrationLimits,
    TaskDescription, TaskStatus, TestCanaryProvider, TransportBinding,
};
use std::sync::Arc;
use std::time::Duration;

fn proxy_endpoint() -> String {
    std::env::var("DXBOT_TEST_OPENAI_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:10000".to_owned())
}

fn task(intent: &str) -> TaskDescription {
    TaskDescription {
        intent: intent.to_string(),
        context: "bounded-context".to_string(),
        budget: Some(50),
        deadline: None,
    }
}

#[test]
fn provider_infra_001_transport_construction() {
    let transport = HttpTransport::new(&proxy_endpoint(), Duration::from_secs(10));
    assert_eq!(transport.base_url, proxy_endpoint());
    assert_eq!(transport.timeout, Duration::from_secs(10));
}

#[test]
#[ignore = "requires OpenAI-compatible proxy via DXBOT_TEST_OPENAI_ENDPOINT (default localhost:10000)"]
fn provider_infra_002_non_streaming_completion() {
    let transport = HttpTransport::new(&proxy_endpoint(), Duration::from_secs(30));
    let protocol = ChatCompletionProtocol::new(transport, 1024 * 1024, 1000);
    let request = ProviderRequest {
        model: "alibaba/deepseek-v4-flash-0731".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Say just the word OK, nothing else.".to_string(),
        }],
        max_tokens: Some(10),
        temperature: Some(0.0),
        stream: false,
    };
    let config = ProviderExecuteConfig {
        deadline: None,
        cancel_token: None,
        max_output_bytes: 1024 * 1024,
        max_output_items: 1000,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio runtime");
    let events = rt
        .block_on(protocol.execute(&request, &config))
        .expect("protocol execute");
    assert!(!events.is_empty(), "should get at least one event");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ProviderEvent::Completed { .. }))
    );
}

#[test]
#[ignore = "requires OpenAI-compatible proxy via DXBOT_TEST_OPENAI_ENDPOINT (default localhost:10000)"]
fn provider_infra_003_streaming_parse() {
    let transport = HttpTransport::new(&proxy_endpoint(), Duration::from_secs(30));
    let protocol = ChatCompletionProtocol::new(transport, 1024 * 1024, 1000);
    let request = ProviderRequest {
        model: "alibaba/deepseek-v4-flash-0731".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Say hello".to_string(),
        }],
        max_tokens: Some(10),
        temperature: Some(0.0),
        stream: true,
    };
    let config = ProviderExecuteConfig {
        deadline: None,
        cancel_token: None,
        max_output_bytes: 1024 * 1024,
        max_output_items: 1000,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio runtime");
    let events = rt
        .block_on(protocol.execute(&request, &config))
        .expect("protocol execute");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, ProviderEvent::Completed { .. }))
    );
}

#[test]
fn provider_infra_004_deadline_exceeded_before_transport() {
    let transport = HttpTransport::new(&proxy_endpoint(), Duration::from_secs(30));
    let protocol = ChatCompletionProtocol::new(transport, 1024 * 1024, 1000);
    let request = ProviderRequest {
        model: "alibaba/deepseek-v4-flash-0731".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "deadline".to_string(),
        }],
        max_tokens: Some(10),
        temperature: Some(0.0),
        stream: false,
    };
    let config = ProviderExecuteConfig {
        deadline: Some(tokio::time::Instant::now()),
        cancel_token: None,
        max_output_bytes: 1024 * 1024,
        max_output_items: 1000,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio runtime");
    let result = rt.block_on(protocol.execute(&request, &config));
    assert!(matches!(result, Err(ProviderError::DeadlineExceeded)));
}

#[test]
#[ignore = "requires OpenAI-compatible proxy via DXBOT_TEST_OPENAI_ENDPOINT (default localhost:10000)"]
fn provider_infra_005_output_bounded() {
    let transport = HttpTransport::new(&proxy_endpoint(), Duration::from_secs(30));
    let protocol = ChatCompletionProtocol::new(transport, 10, 2);
    let request = ProviderRequest {
        model: "alibaba/deepseek-v4-flash-0731".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Write a long response".to_string(),
        }],
        max_tokens: Some(200),
        temperature: Some(0.0),
        stream: false,
    };
    let config = ProviderExecuteConfig {
        deadline: None,
        cancel_token: None,
        max_output_bytes: 10,
        max_output_items: 2,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio runtime");
    let result = rt.block_on(protocol.execute(&request, &config));
    assert!(matches!(result, Err(ProviderError::OutputExceeded)));
}

#[test]
fn provider_infra_006_transport_unavailable() {
    let transport = HttpTransport::new("http://localhost:19999", Duration::from_secs(2));
    let protocol = ChatCompletionProtocol::new(transport, 1024 * 1024, 1000);
    let request = ProviderRequest {
        model: "alibaba/deepseek-v4-flash-0731".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "test".to_string(),
        }],
        max_tokens: Some(10),
        temperature: Some(0.0),
        stream: false,
    };
    let config = ProviderExecuteConfig {
        deadline: None,
        cancel_token: None,
        max_output_bytes: 1024 * 1024,
        max_output_items: 1000,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio runtime");
    let result = rt.block_on(protocol.execute(&request, &config));
    assert!(matches!(
        result,
        Err(ProviderError::TransportUnavailable { .. })
    ));
}

#[test]
fn provider_infra_007_transport_unavailable_fails_closed_without_fallback() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let transport = HttpTransport::new("http://localhost:19999", Duration::from_secs(2));
    let binding = TransportBinding::new(
        ProtocolKind::OpenAiChatCompletions,
        transport,
        1024 * 1024,
        1000,
    );
    let provider = HttpExecuteProvider::new(
        Box::new(DeepSeekFlashAdapter::new(
            ProviderId("deepseek-flash".to_string()),
            "llm-chat",
            1,
        )),
        &binding,
        1024 * 1024,
        1000,
    );
    let host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    host.register(ProviderRegistration::new(
        provider as Arc<_>,
        ProtocolKind::OpenAiChatCompletions,
        binding,
        RegistrationLimits {
            max_output_bytes: 1024 * 1024,
            max_output_items: 1000,
        },
    ))
    .unwrap();
    // Fails closed with a typed transport error; there is no reference fallback.
    let error = host
        .execute_task_with_cancel(
            &ProviderId("deepseek-flash".to_string()),
            1,
            &task("write a plan"),
            CancellationToken::new(),
        )
        .expect_err("must fail closed");
    assert!(matches!(
        error,
        provider_host::HarnessError::TransportUnavailable { .. }
    ));
}

#[test]
fn provider_infra_008_canary_executes_through_common_host() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let mut host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    TestCanaryProvider::new(ProviderId("canary-1".to_string()), "text", 2)
        .register_into(&mut host)
        .unwrap();
    let result = host
        .execute_task_with_cancel(
            &ProviderId("canary-1".to_string()),
            2,
            &task("write a plan"),
            CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(result.output, "canary:write a plan");
    assert_eq!(result.status, TaskStatus::Completed);
}

#[test]
fn provider_common_cancel_preempts_transport() {
    let transport = HttpTransport::new(&proxy_endpoint(), Duration::from_secs(30));
    let protocol = ChatCompletionProtocol::new(transport, 1024, 16);
    let request = ProviderRequest {
        model: "cancel-test".to_string(),
        messages: Vec::new(),
        max_tokens: None,
        temperature: None,
        stream: false,
    };
    let token = CancellationToken::new();
    token.cancel();
    let config = ProviderExecuteConfig {
        deadline: None,
        cancel_token: Some(token),
        max_output_bytes: 1024,
        max_output_items: 16,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("tokio runtime");
    let result = rt.block_on(protocol.execute(&request, &config));
    assert!(matches!(result, Err(ProviderError::Cancelled)));
}
