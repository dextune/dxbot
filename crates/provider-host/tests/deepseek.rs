//! Acceptance tests for DeepSeek v4 Flash adapter (AT-DEEPSEEK-001..005).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use dxbot_core::types::ProviderId;
use provider_host::{
    ChatCompletionProtocol, ChatMessage, DeepSeekFlashAdapter, HttpTransport, ProviderEvent,
    ProviderExecuteConfig, ProviderHost, ProviderRequest, RealProvider, ReferenceProvider,
    TaskDescription, TaskStatus,
};
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
fn deepseek_001_implements_real_provider_trait() {
    let adapter = DeepSeekFlashAdapter::new(ProviderId("deepseek-flash".to_string()), "text", 1);
    assert_eq!(adapter.id(), &ProviderId("deepseek-flash".to_string()));
    assert_eq!(adapter.capability(), "text");
    assert_eq!(adapter.generation(), 1);

    let req = adapter.build_request(&task("Say hello"));
    assert_eq!(req.model, "alibaba/deepseek-v4-flash-0731");
    assert_eq!(req.temperature, Some(0.0));
    assert!(!req.messages.is_empty());
    assert_eq!(req.messages[0].role, "user");
    assert!(req.messages[0].content.contains("Say hello"));

    let oversized = TaskDescription {
        budget: Some(u64::MAX),
        ..task("bounded conversion")
    };
    assert_eq!(adapter.build_request(&oversized).max_tokens, Some(u32::MAX));
}

#[test]
#[ignore = "requires OpenAI-compatible proxy via DXBOT_TEST_OPENAI_ENDPOINT (default localhost:10000)"]
fn deepseek_002_real_proxy_produces_output() {
    let mut host = ProviderHost::new();
    host.set_transport(HttpTransport::new(
        &proxy_endpoint(),
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
    assert_eq!(result.status, TaskStatus::Completed);
}

#[test]
#[ignore = "requires OpenAI-compatible proxy via DXBOT_TEST_OPENAI_ENDPOINT (default localhost:10000)"]
fn deepseek_003_reasoning_tokens_in_stream() {
    let transport = HttpTransport::new(&proxy_endpoint(), Duration::from_secs(30));
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
fn deepseek_004_chain_real_adapter_preferred() {
    let mut host = ProviderHost::new();
    host.set_transport(HttpTransport::new(
        &proxy_endpoint(),
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
    assert!(!result.output.starts_with("adapter:"));
    assert_eq!(result.status, TaskStatus::Completed);
}

#[test]
#[ignore = "requires OpenAI-compatible proxy via DXBOT_TEST_OPENAI_ENDPOINT (default localhost:10000)"]
fn deepseek_005_temperature_zero_is_deterministic() {
    let mut host = ProviderHost::new();
    host.set_transport(HttpTransport::new(
        &proxy_endpoint(),
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
        context: String::new(),
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
