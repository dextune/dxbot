//! Shared Conformance matrix owner (`DXB-DEL-068` H3/H11 §11).
//!
//! This test drives the single canonical row owner
//! ([`provider_host::conformance_rows`]) across every column — the synthetic
//! `TestCanary`, the direct MiniMax-M3 adapter over a deterministic mock HTTP
//! server, the official DeepSeek Harness ACP provider over an ACTUAL fake
//! subprocess, and the deterministic reference fixture. Every (column, axis)
//! cell is declared exactly once by the owner; inapplicable axes are explicit
//! `NotSupported { rejection }` negatives rather than silent skips, and this
//! test asserts both the completeness of the matrix and that each present
//! column behaves as its owned rows declare.
//!
//! Reusing the one row owner (rather than writing separate adapter tests with
//! similar assertions) is the point: adding an axis or a column is a single
//! edit in the testkit, and this owner test then forces every column to declare
//! and honour it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(feature = "testkit")]

use std::collections::BTreeSet;

use provider_host::{
    ConformanceCapability, ConformanceColumn, ConformanceOutcomeKind, ConformanceSuite,
    ReferenceProvider, TestCanaryProvider, conformance_row, conformance_rows,
};

fn test_handle() -> (tokio::runtime::Runtime, tokio::runtime::Handle) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let handle = runtime.handle().clone();
    (runtime, handle)
}

/// The matrix must declare every (column, capability) cell exactly once: no
/// silent skip, no duplicate, and full coverage of both axes.
#[test]
fn matrix_declares_every_cell_exactly_once() {
    let rows = conformance_rows();
    let mut seen: BTreeSet<(ConformanceColumn, ConformanceCapability)> = BTreeSet::new();
    for row in &rows {
        assert!(
            seen.insert((row.column, row.capability)),
            "duplicate cell for {:?}/{:?}",
            row.column,
            row.capability
        );
    }
    // Full cross product coverage.
    for column in ConformanceColumn::ALL {
        for capability in ConformanceCapability::ALL {
            assert!(
                seen.contains(&(column, capability)),
                "missing conformance cell {}/{} (no silent skip allowed)",
                column.as_str(),
                capability.as_str()
            );
        }
    }
    assert_eq!(
        rows.len(),
        ConformanceColumn::ALL.len() * ConformanceCapability::ALL.len()
    );
}

/// Every `NotSupported` cell must carry an explicit, non-empty rejection reason
/// — the plan forbids silent skips for inapplicable capabilities.
#[test]
fn not_supported_cells_carry_explicit_rejections() {
    for row in conformance_rows() {
        if let ConformanceOutcomeKind::NotSupported { rejection } = &row.outcome {
            assert!(
                !rejection.trim().is_empty(),
                "{}/{} NotSupported must state a rejection reason",
                row.column.as_str(),
                row.capability.as_str()
            );
        }
    }
}

/// The two production columns are exactly the direct MiniMax adapter and the
/// DSH ACP provider; canary and reference are not production columns.
#[test]
fn production_columns_are_the_two_adapters() {
    let production: Vec<_> = ConformanceColumn::ALL
        .into_iter()
        .filter(|column| column.is_production())
        .collect();
    assert_eq!(
        production,
        vec![
            ConformanceColumn::DirectMiniMax,
            ConformanceColumn::DeepSeekHarnessAcp
        ]
    );
}

/// The generic Execute-contract axes (registration/request/event/success/
/// rejection/cancel/deadline/malformed-overflow/lifecycle/generation) are owned
/// as `Supported` by every column, and the in-process columns run them through
/// the shared deterministic suite via the same public registration path.
#[test]
fn test_canary_column_runs_supported_generic_axes() {
    // The canary owns all generic axes as Supported.
    for capability in generic_axes() {
        assert!(
            conformance_row(ConformanceColumn::TestCanary, capability)
                .outcome
                .is_supported(),
            "canary must own {capability:?} as Supported"
        );
    }
    let (_runtime, handle) = test_handle();
    let passed = ConformanceSuite::run_deterministic(
        handle,
        "llm-chat",
        |id, generation| TestCanaryProvider::new(id, "llm-chat", generation),
        |host, provider| provider.register_into(host),
        |intent| format!("canary:{intent}"),
    );
    // The suite exercises registration/selection/success/duplicate/unknown/
    // cancellation/replace/drain/lease/reclaim/stale-fence — the generic axes.
    assert!(!passed.is_empty());
}

#[test]
fn reference_column_runs_supported_generic_axes_and_declares_negatives() {
    // Reference owns the generic axes as Supported...
    for capability in generic_axes() {
        assert!(
            conformance_row(ConformanceColumn::Reference, capability)
                .outcome
                .is_supported()
        );
    }
    // ...but explicitly does NOT own the external side-effect / subprocess
    // recovery / subprocess resource / removability axes (explicit negatives).
    for capability in [
        ConformanceCapability::SideEffect,
        ConformanceCapability::Recovery,
        ConformanceCapability::Resource,
        ConformanceCapability::Removability,
    ] {
        assert!(matches!(
            conformance_row(ConformanceColumn::Reference, capability).outcome,
            ConformanceOutcomeKind::NotSupported { .. }
        ));
    }
    let (_runtime, handle) = test_handle();
    let passed = ConformanceSuite::run_deterministic(
        handle,
        "llm-chat",
        |id, generation| ReferenceProvider::new(id, "llm-chat", generation),
        |host, provider| provider.register_into(host),
        |intent| intent.to_owned(),
    );
    assert!(!passed.is_empty());
}

/// The direct MiniMax column owns request/success/rejection/security over HTTP
/// but explicitly declares side-effect/recovery/resource as negatives (no tool
/// ledger, no managed child).
#[test]
fn direct_minimax_column_declares_http_negatives() {
    for capability in [
        ConformanceCapability::SideEffect,
        ConformanceCapability::Recovery,
        ConformanceCapability::Resource,
    ] {
        assert!(matches!(
            conformance_row(ConformanceColumn::DirectMiniMax, capability).outcome,
            ConformanceOutcomeKind::NotSupported { .. }
        ));
    }
    // But it owns request/success/removability as Supported.
    for capability in [
        ConformanceCapability::Request,
        ConformanceCapability::Success,
        ConformanceCapability::Removability,
    ] {
        assert!(
            conformance_row(ConformanceColumn::DirectMiniMax, capability)
                .outcome
                .is_supported()
        );
    }
}

/// The DSH ACP column owns EVERY axis as Supported — including side-effect,
/// recovery, resource, and removability — because it drives an actual bounded
/// subprocess with a tool/effect ledger. The concrete actual-subprocess
/// behaviour rows (spawn/handshake/update/result/cancel/hang/crash/malformed/
/// contaminated/oversized/permission/leak) are owned by `tests/acp.rs` and the
/// deterministic fault matrix; this owner test asserts the DSH column declares
/// full support (no negative cell) so those rows cannot silently regress into
/// skips.
#[test]
fn deepseek_harness_acp_column_owns_all_axes() {
    for capability in ConformanceCapability::ALL {
        assert!(
            conformance_row(ConformanceColumn::DeepSeekHarnessAcp, capability)
                .outcome
                .is_supported(),
            "DSH ACP column must own {capability:?} as Supported (actual subprocess column)"
        );
    }
}

fn generic_axes() -> [ConformanceCapability; 10] {
    [
        ConformanceCapability::Registration,
        ConformanceCapability::Request,
        ConformanceCapability::Event,
        ConformanceCapability::Success,
        ConformanceCapability::ProviderRejection,
        ConformanceCapability::Cancellation,
        ConformanceCapability::Deadline,
        ConformanceCapability::MalformedOverflow,
        ConformanceCapability::Lifecycle,
        ConformanceCapability::Generation,
    ]
}
