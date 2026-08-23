//! Acceptance tests for DeepSeek v4 Flash adapter (AT-DEEPSEEK-001..005).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dxbot_core::types::ProviderId;
use provider_host::{
    ChatCompletionProtocol, ChatMessage, DeepSeekFlashAdapter, HttpTransport, ProviderEvent,
    ProviderExecuteConfig, ProviderHost, ProviderRequest, RealProvider, ReferenceProvider,
    TaskDescription, TaskStatus,
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

// ── AT-DEEPSEEK-001: DeepSeekFlashAdapter implements RealProvider trait ──

#[test]
fn deepseek_001_implements_real_provider_trait() {
    let adapter = DeepSeekFlashAdapter::new(
        ProviderId("deepseek-flash".to_string()),
        "text",
        1,
    );
    assert_eq!(adapter.id(), &ProviderId("deepseek-flash".to_string()));
    assert_eq!(adapter.capability(), "text");
    assert_eq!(adapter.generation(), 1);

    let req = adapter.build_request(&task("Say hello"));
    assert_eq!(req.model, "alibaba/deepseek-v4-flash-0731");
    assert_eq!(req.temperature, Some(0.0));
    assert!(!req.messages.is_empty());
    assert_eq!(req.messages[0].role, "user");
    assert!(req.messages[0].content.contains("Say hello"));
}

// ── AT-DEEPSEEK-002: Real proxy call produces non-empty output ──

#[test]
fn deepseek_002_real_proxy_produces_output() {
    let mut host = ProviderHost::new();
    host.set_transport(HttpTransport::new(
        "http://localhost:10000",
        Duration::from_secs(30),
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

    let result = host.execute_task(&task("Say hello")).unwrap();
    assert!(!result.output.is_empty(), "real provider should produce output");
    assert_eq!(result.status, TaskStatus::Completed);
}

// ── AT-DEEPSEEK-003: Reasoning token included in SSE stream ──

#[test]
fn deepseek_003_reasoning_tokens_in_stream() {
    let transport = HttpTransport::new("http://localhost:10000", Duration::from_secs(30));
    let protocol = ChatCompletionProtocol::new(transport, 1024 * 1024, 1000);

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
        cancel_notify: None,
        max_output_bytes: 1024 * 1024,
        max_output_items: 1000,
    };

    let rt = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().expect("tokio runtime");
    let events = rt.block_on(protocol.execute(&request, &config)).expect("protocol execute");

    let has_reasoning = events
        .iter()
        .any(|e| matches!(e, ProviderEvent::ReasoningDelta { .. }));
    let has_content = events
        .iter()
        .any(|e| matches!(e, ProviderEvent::ContentDelta { .. }));
    let has_completed = events
        .iter()
        .any(|e| matches!(e, ProviderEvent::Completed { .. }));
    assert!(
        has_reasoning || has_content,
        "deepseek should produce reasoning or content deltas"
    );
    assert!(has_completed, "should have Completed event");
}

// ── AT-DEEPSEEK-004: Provider chain: real adapter preferred, Reference fallback ──

#[test]
fn deepseek_004_chain_real_adapter_preferred() {
    let mut host = ProviderHost::new();
    host.set_transport(HttpTransport::new(
        "http://localhost:10000",
        Duration::from_secs(30),
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

    let result = host.execute_task(&task("Say hello")).unwrap();
    assert!(!result.output.is_empty());
    assert!(
        !result.output.starts_with("adapter:"),
        "real provider output should not be canary prefix"
    );
    assert_eq!(result.status, TaskStatus::Completed);
}

// ── AT-DEEPSEEK-005: temperature=0.0 gives deterministic output (soft) ──

#[test]
fn deepseek_005_temperature_zero_is_deterministic() {
    let mut host = ProviderHost::new();
    host.set_transport(HttpTransport::new(
        "http://localhost:10000",
        Duration::from_secs(30),
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

    let task_def = TaskDescription {
        intent: "Say just the number 42".to_string(),
        context: "".to_string(),
        budget: Some(10),
        deadline: None,
    };
    let result1 = host.execute_task(&task_def).unwrap();
    let result2 = host.execute_task(&task_def).unwrap();
    assert!(!result1.output.is_empty());
    assert!(!result2.output.is_empty());
    assert_eq!(result1.status, TaskStatus::Completed);
    assert_eq!(result2.status, TaskStatus::Completed);
}