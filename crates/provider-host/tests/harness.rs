//! Provider host acceptance coverage (`DXB-DEL-068` H1–H4).
//!
//! Exercises the canonical async Execute contract, exact id+generation
//! selection with no fallback, per-registration protocol/transport isolation,
//! atomic replace/drain/unregister semantics, and the shared Conformance suite
//! run over both the synthetic canary and the deterministic reference.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#![cfg(all(feature = "direct-deepseek", feature = "direct-minimax"))]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use dxbot_core::types::ProviderId;
use provider_host::{
    CancellationToken, ConformanceCase, ConformanceSuite, DeepSeekFlashAdapter, HarnessError,
    HttpExecuteProvider, HttpTransport, MiniMaxM3Adapter, ProtocolKind, ProviderHost, ProviderInfo,
    ProviderRegistration, ProviderStatus, ReferenceProvider, RegistrationLimits, TaskDescription,
    TaskStatus, TestCanaryProvider, TransportBinding,
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
fn canary_executes_through_canonical_contract() {
    let runtime = test_runtime();
    let mut host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    TestCanaryProvider::new(ProviderId("canary-1".to_owned()), "text", 2)
        .register_into(&mut host)
        .unwrap();

    let result = host
        .execute_task_with_cancel(
            &ProviderId("canary-1".to_owned()),
            2,
            &task("write a plan"),
            CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(result.output, "canary:write a plan");
    assert_eq!(result.status, TaskStatus::Completed);
    assert!(
        result
            .evidence
            .iter()
            .any(|evidence| evidence.provider == ProviderId("canary-1".to_owned()))
    );
}

#[test]
fn reference_fixture_executes_through_canonical_contract() {
    let runtime = test_runtime();
    let mut host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    ReferenceProvider::new(ProviderId("ref-1".to_owned()), "text", 1)
        .register_into(&mut host)
        .unwrap();
    let result = host
        .execute_task_with_cancel(
            &ProviderId("ref-1".to_owned()),
            1,
            &task("write a plan"),
            CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(result.output, "write a plan");
    assert_eq!(result.status, TaskStatus::Completed);
}

#[test]
fn unknown_or_replaced_provider_fails_closed_without_fallback() {
    let runtime = test_runtime();
    let mut host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    ReferenceProvider::new(ProviderId("ref-1".to_owned()), "text", 1)
        .register_into(&mut host)
        .unwrap();

    // A different id is not silently substituted by the registered reference.
    let unknown = host.execute_task_with_cancel(
        &ProviderId("other".to_owned()),
        1,
        &task("x"),
        CancellationToken::new(),
    );
    assert!(matches!(
        unknown,
        Err(HarnessError::ProviderNotFound { .. })
    ));

    // The wrong generation for a known id is fenced, not substituted.
    let fenced = host.execute_task_with_cancel(
        &ProviderId("ref-1".to_owned()),
        99,
        &task("x"),
        CancellationToken::new(),
    );
    assert!(matches!(fenced, Err(HarnessError::GenerationFenced { .. })));
}

#[test]
fn list_and_get_reflect_active_registrations() {
    let runtime = test_runtime();
    let mut host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    ReferenceProvider::new(ProviderId("ref-text".to_owned()), "text", 1)
        .register_into(&mut host)
        .unwrap();
    TestCanaryProvider::new(ProviderId("canary-code".to_owned()), "code", 2)
        .register_into(&mut host)
        .unwrap();

    assert_eq!(host.list_providers(None).len(), 2);
    let text = host.list_providers(Some("text"));
    assert_eq!(text.len(), 1);
    assert_eq!(text[0].id, ProviderId("ref-text".to_owned()));
    let code = host.list_providers(Some("code"));
    assert_eq!(code.len(), 1);
    assert_eq!(code[0].id, ProviderId("canary-code".to_owned()));
    assert!(host.list_providers(Some("vision")).is_empty());

    let info = host
        .get_provider(&ProviderId("canary-code".to_owned()))
        .unwrap();
    assert_eq!(info.capability, "code");
    assert_eq!(info.status, ProviderStatus::Ready);
    assert!(matches!(
        host.get_provider(&ProviderId("missing".to_owned())),
        Err(HarnessError::ProviderNotFound { .. })
    ));
}

#[test]
fn replace_publishes_new_generation_and_drains_old() {
    let runtime = test_runtime();
    let mut host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    let id = ProviderId("canary".to_owned());
    TestCanaryProvider::new(id.clone(), "text", 1)
        .register_into(&mut host)
        .unwrap();

    let replacement = ProviderRegistration::detached(
        Arc::new(TestCanaryProvider::new(id.clone(), "text", 2)),
        ProtocolKind::OpenAiChatCompletions,
        RegistrationLimits {
            max_output_bytes: 64 * 1024,
            max_output_items: 256,
        },
    );
    host.replace(replacement).unwrap();

    // Active generation is now 2 (Ready). New admission targets generation 2.
    let active = host.get_provider(&id).unwrap();
    assert_eq!(active.generation, 2);
    assert_eq!(active.status, ProviderStatus::Ready);

    // A replacement whose generation is not strictly greater is rejected.
    let stale = ProviderRegistration::detached(
        Arc::new(TestCanaryProvider::new(id.clone(), "text", 2)),
        ProtocolKind::OpenAiChatCompletions,
        RegistrationLimits {
            max_output_bytes: 64 * 1024,
            max_output_items: 256,
        },
    );
    assert!(matches!(
        host.replace(stale),
        Err(HarnessError::InvalidProvider { .. })
    ));

    // The old generation was a zero-lease Draining slot, reclaimed on replace;
    // a call beginning after replace against it is fenced (no new admission on a
    // superseded generation). In-flight work admitted before replace keeps its
    // captured provider+activity and may still finish, but a fresh call cannot
    // re-enter generation 1.
    let old = host.execute_task_with_cancel(&id, 1, &task("old"), CancellationToken::new());
    assert!(matches!(old, Err(HarnessError::GenerationFenced { .. })));
}

#[test]
fn unregister_captures_old_generation_and_fails_closed_afterwards() {
    let runtime = test_runtime();
    let mut host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    let id = ProviderId("canary".to_owned());
    TestCanaryProvider::new(id.clone(), "text", 5)
        .register_into(&mut host)
        .unwrap();

    let captured = host.unregister(&id).unwrap();
    assert_eq!(captured, 5);
    assert!(host.list_providers(None).is_empty());
    assert!(matches!(
        host.execute_task_with_cancel(&id, 5, &task("x"), CancellationToken::new()),
        Err(HarnessError::ProviderNotFound { .. })
    ));
    assert!(matches!(
        host.unregister(&id),
        Err(HarnessError::ProviderNotFound { .. })
    ));
}

#[test]
fn drain_stops_new_admission_and_reports_draining() {
    let runtime = test_runtime();
    let mut host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    let id = ProviderId("canary".to_owned());
    TestCanaryProvider::new(id.clone(), "text", 3)
        .register_into(&mut host)
        .unwrap();
    let generation = host.begin_drain(&id).unwrap();
    assert_eq!(generation, 3);
    // A zero-lease drained generation is reclaimed, so the id no longer has a
    // selectable slot and `get_provider` reports it gone.
    assert!(matches!(
        host.get_provider(&id),
        Err(HarnessError::ProviderNotFound { .. })
    ));
    // A call beginning after drain is fenced: no new admission on a drained
    // generation. In-flight work admitted before the drain keeps its captured
    // provider+activity and may still complete, but a fresh call cannot enter.
    let result = host.execute_task_with_cancel(&id, 3, &task("drain"), CancellationToken::new());
    assert!(matches!(result, Err(HarnessError::ProviderNotFound { .. })));
}

/// H4: two registrations with different protocol/transport bindings coexist and
/// execute over their own captured binding without cross-route contamination.
#[cfg(unix)]
#[test]
fn per_registration_protocol_transport_isolation() {
    // Anthropic-bound MiniMax registration.
    let anthropic_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let anthropic_endpoint = format!(
        "http://{}/anthropic/v1",
        anthropic_listener.local_addr().unwrap()
    );
    let anthropic_server = std::thread::spawn(move || {
        let (mut stream, _) = anthropic_listener.accept().unwrap();
        let mut buf = vec![0_u8; 16 * 1024];
        let size = stream.read(&mut buf).unwrap();
        buf.truncate(size);
        let text = String::from_utf8_lossy(&buf);
        assert!(text.starts_with("POST /anthropic/v1/messages HTTP/1.1"));
        assert!(
            text.to_ascii_lowercase()
                .contains("x-api-key: anthropic-secret")
        );
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "ANTHROPIC_ROUTE_OK"}],
            "usage": {"input_tokens": 1, "output_tokens": 1}
        })
        .to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
        stream.flush().unwrap();
    });

    // OpenAI-bound DeepSeek registration.
    let openai_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let openai_endpoint = format!("http://{}", openai_listener.local_addr().unwrap());
    let openai_server = std::thread::spawn(move || {
        let (mut stream, _) = openai_listener.accept().unwrap();
        let mut buf = vec![0_u8; 16 * 1024];
        let size = stream.read(&mut buf).unwrap();
        buf.truncate(size);
        let text = String::from_utf8_lossy(&buf);
        assert!(text.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(
            text.to_ascii_lowercase()
                .contains("authorization: bearer openai-secret")
        );
        let body = serde_json::json!({
            "choices": [{"message": {"content": "OPENAI_ROUTE_OK", "reasoning_content": null}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "reasoning_tokens": 0}
        })
        .to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
        stream.flush().unwrap();
    });

    let runtime = test_runtime();
    let host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());

    let anthropic_transport = HttpTransport::new(&anthropic_endpoint, Duration::from_secs(2))
        .with_anthropic_key("anthropic-secret")
        .unwrap();
    let anthropic_binding = TransportBinding::new(
        ProtocolKind::AnthropicMessages,
        anthropic_transport,
        MAX_BYTES,
        MAX_ITEMS,
    );
    let anthropic_provider = HttpExecuteProvider::new(
        Box::new(MiniMaxM3Adapter::new(
            ProviderId("minimax".to_owned()),
            "llm-chat",
            1,
        )),
        &anthropic_binding,
        MAX_BYTES,
        MAX_ITEMS,
    );
    host.register(ProviderRegistration::new(
        anthropic_provider as Arc<_>,
        ProtocolKind::AnthropicMessages,
        anthropic_binding,
        RegistrationLimits {
            max_output_bytes: MAX_BYTES,
            max_output_items: MAX_ITEMS,
        },
    ))
    .unwrap();

    let openai_transport = HttpTransport::new(&openai_endpoint, Duration::from_secs(2))
        .with_bearer("openai-secret")
        .unwrap();
    let openai_binding = TransportBinding::new(
        ProtocolKind::OpenAiChatCompletions,
        openai_transport,
        MAX_BYTES,
        MAX_ITEMS,
    );
    let openai_provider = HttpExecuteProvider::new(
        Box::new(DeepSeekFlashAdapter::new(
            ProviderId("deepseek".to_owned()),
            "llm-chat",
            1,
        )),
        &openai_binding,
        MAX_BYTES,
        MAX_ITEMS,
    );
    host.register(ProviderRegistration::new(
        openai_provider as Arc<_>,
        ProtocolKind::OpenAiChatCompletions,
        openai_binding,
        RegistrationLimits {
            max_output_bytes: MAX_BYTES,
            max_output_items: MAX_ITEMS,
        },
    ))
    .unwrap();

    let anthropic_result = host
        .execute_task_with_cancel(
            &ProviderId("minimax".to_owned()),
            1,
            &task("route"),
            CancellationToken::new(),
        )
        .unwrap();
    let openai_result = host
        .execute_task_with_cancel(
            &ProviderId("deepseek".to_owned()),
            1,
            &task("route"),
            CancellationToken::new(),
        )
        .unwrap();
    anthropic_server.join().unwrap();
    openai_server.join().unwrap();

    // Each registration used its own captured protocol/transport binding.
    assert_eq!(anthropic_result.output, "ANTHROPIC_ROUTE_OK");
    assert_eq!(openai_result.output, "OPENAI_ROUTE_OK");
}

/// H3: the shared Conformance suite runs the same rows over the canary and the
/// reference through the same public registration + execute path.
#[test]
fn shared_conformance_suite_covers_canary_and_reference() {
    let runtime = test_runtime();
    let expected_cases = [
        ConformanceCase::Registration,
        ConformanceCase::ExactGenerationSelection,
        ConformanceCase::SuccessTerminal,
        ConformanceCase::DuplicateRejection,
        ConformanceCase::UnknownRejection,
        ConformanceCase::Cancellation,
        ConformanceCase::ReplaceFencesOldGeneration,
        ConformanceCase::DrainStopsNewAdmission,
        ConformanceCase::ActivityLeaseLifecycle,
        ConformanceCase::SlotReclamationBounded,
        ConformanceCase::StaleGenerationCommitFenced,
    ];

    let canary_cases = ConformanceSuite::run_deterministic(
        runtime.handle().clone(),
        "text",
        |id, generation| TestCanaryProvider::new(id, "text", generation),
        |host, provider| provider.register_into(host),
        |intent| format!("canary:{intent}"),
    );
    assert_eq!(canary_cases, expected_cases);

    let reference_cases = ConformanceSuite::run_deterministic(
        runtime.handle().clone(),
        "text",
        |id, generation| ReferenceProvider::new(id, "text", generation),
        |host, provider| provider.register_into(host),
        |intent| intent.to_owned(),
    );
    assert_eq!(reference_cases, expected_cases);
}

/// Diagnostics/list surface stays additive-compatible: `ProviderInfo` still
/// exposes id/capability/generation/status.
#[test]
fn provider_info_shape_is_stable() {
    let info = ProviderInfo {
        id: ProviderId("p".to_owned()),
        capability: "llm-chat".to_owned(),
        generation: 1,
        status: ProviderStatus::Ready,
    };
    assert_eq!(info.id.0, "p");
    assert_eq!(info.capability, "llm-chat");
    assert_eq!(info.generation, 1);
    assert_eq!(info.status, ProviderStatus::Ready);
}
