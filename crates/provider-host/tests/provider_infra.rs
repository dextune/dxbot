//! Acceptance tests for provider common infrastructure (AT-PROVIDER-INFRA-001..008).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dxbot_core::types::ProviderId;
use provider_host::{
    CancellationToken, ChatCompletionProtocol, ChatMessage, DeepSeekFlashAdapter, HarnessAdapter,
    HttpTransport, ProviderError, ProviderEvent, ProviderExecuteConfig, ProviderHost,
    ProviderRequest, ReferenceProvider, TaskDescription, TaskStatus,
};
use std::time::Duration;

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
    let transport = HttpTransport::new("http://localhost:10000", Duration::from_secs(10));
    assert_eq!(transport.base_url, "http://localhost:10000");
    assert_eq!(transport.timeout, Duration::from_secs(10));
}

#[test]
#[ignore = "requires local OpenAI-compatible proxy on localhost:10000"]
fn provider_infra_002_non_streaming_completion() {
    let transport = HttpTransport::new("http://localhost:10000", Duration::from_secs(30));
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
#[ignore = "requires local OpenAI-compatible proxy on localhost:10000"]
fn provider_infra_003_streaming_parse() {
    let transport = HttpTransport::new("http://localhost:10000", Duration::from_secs(30));
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
    let transport = HttpTransport::new("http://localhost:10000", Duration::from_secs(30));
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
#[ignore = "requires local OpenAI-compatible proxy on localhost:10000"]
fn provider_infra_005_output_bounded() {
    let transport = HttpTransport::new("http://localhost:10000", Duration::from_secs(30));
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
fn provider_infra_007_chain_fallback_to_reference() {
    let mut host = ProviderHost::new();
    host.set_transport(HttpTransport::new(
        "http://localhost:19999",
        Duration::from_secs(2),
    ));
    host.register_real_provider(Box::new(DeepSeekFlashAdapter::new(
        ProviderId("deepseek-flash".to_string()),
        "text",
        1,
    )))
    .unwrap();
    host.register_reference_provider(ReferenceProvider::new(
        ProviderId("ref-1".to_string()),
        "text",
        1,
    ))
    .unwrap();
    let result = host.execute_task(&task("write a plan")).unwrap();
    assert_eq!(result.output, "write a plan");
    assert_eq!(result.status, TaskStatus::Completed);
    assert!(
        result
            .evidence
            .iter()
            .any(|evidence| evidence.provider == ProviderId("ref-1".to_string()))
    );
}

#[test]
fn provider_infra_008_existing_canary_still_works() {
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
    let result = host.execute_task(&task("write a plan")).unwrap();
    assert_eq!(result.output, "adapter:write a plan");
    assert_eq!(result.status, TaskStatus::Completed);
}

#[test]
fn provider_common_cancel_preempts_transport() {
    let transport = HttpTransport::new("http://localhost:10000", Duration::from_secs(30));
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
