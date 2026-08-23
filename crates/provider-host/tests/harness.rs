//! Acceptance coverage for `AT-HARNESS-001`: Reference Provider plus one real
//! Harness Adapter canary.
#![allow(clippy::unwrap_used)]

use dxbot_core::types::ProviderId;
use provider_host::{
    HarnessAdapter, HarnessError, ProviderHost, ProviderStatus, ReferenceProvider,
    TaskDescription, TaskStatus,
};

fn task(intent: &str) -> TaskDescription {
    TaskDescription {
        intent: intent.to_string(),
        context: "bounded-context".to_string(),
        budget: Some(10),
        deadline: Some(1_700_000_000),
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
    assert!(result
        .evidence
        .iter()
        .any(|e| e.provider == ProviderId("ref-1".to_string())));
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
    assert!(result
        .evidence
        .iter()
        .any(|e| e.provider == ProviderId("adapter-1".to_string())
            && e.observation == "harness-adapter-canary"));
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
    assert!(result
        .evidence
        .iter()
        .any(|e| e.provider == ProviderId("ref-1".to_string())));
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

    let info = host.get_provider(&ProviderId("adapter-code".to_string())).unwrap();
    assert_eq!(info.capability, "code");
    assert_eq!(info.status, ProviderStatus::Ready);
    assert!(matches!(
        host.get_provider(&ProviderId("missing".to_string())),
        Err(HarnessError::ProviderNotFound { .. })
    ));
}