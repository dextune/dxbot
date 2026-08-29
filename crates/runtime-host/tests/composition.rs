//! A1 deterministic composition acceptance tests.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use runtime_host::{
    CompositionError, ExtensionKind, ExtensionReadiness, RuntimeComposition, RuntimeExtension,
};

#[derive(Debug)]
struct TestExtension {
    id: String,
    dependencies: Vec<String>,
    start_result: Result<ExtensionReadiness, String>,
    stop_fails: bool,
    log: Arc<Mutex<Vec<String>>>,
}

impl TestExtension {
    fn ready(id: &str, dependencies: &[&str], log: &Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            id: id.to_owned(),
            dependencies: dependencies
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            start_result: Ok(ExtensionReadiness::Ready),
            stop_fails: false,
            log: Arc::clone(log),
        }
    }
}

impl RuntimeExtension for TestExtension {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::InternalService
    }

    fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    fn start(&mut self) -> Result<ExtensionReadiness, String> {
        self.log
            .lock()
            .expect("test log")
            .push(format!("start:{}", self.id));
        self.start_result.clone()
    }

    fn stop(&mut self) -> Result<(), String> {
        self.log
            .lock()
            .expect("test log")
            .push(format!("stop:{}", self.id));
        if self.stop_fails {
            Err("teardown failed".to_owned())
        } else {
            Ok(())
        }
    }
}

#[test]
fn resolution_is_deterministic_and_dependency_ordered() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut first = RuntimeComposition::new();
    first
        .register(Box::new(TestExtension::ready(
            "worker",
            &["provider"],
            &log,
        )))
        .expect("register worker");
    first
        .register(Box::new(TestExtension::ready("provider", &["audit"], &log)))
        .expect("register provider");
    first
        .register(Box::new(TestExtension::ready("audit", &[], &log)))
        .expect("register audit");

    let mut second = RuntimeComposition::new();
    second
        .register(Box::new(TestExtension::ready("audit", &[], &log)))
        .expect("register audit");
    second
        .register(Box::new(TestExtension::ready(
            "worker",
            &["provider"],
            &log,
        )))
        .expect("register worker");
    second
        .register(Box::new(TestExtension::ready("provider", &["audit"], &log)))
        .expect("register provider");

    assert_eq!(
        first.resolve().expect("resolve first"),
        vec!["audit", "provider", "worker"]
    );
    assert_eq!(
        first.resolved_ids(),
        second.resolve().expect("resolve second")
    );
}

#[test]
fn invalid_graph_is_rejected_before_any_start() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut composition = RuntimeComposition::new();
    composition
        .register(Box::new(TestExtension::ready("a", &["missing"], &log)))
        .expect("register a");
    assert!(matches!(
        composition.start(),
        Err(CompositionError::MissingDependency { .. })
    ));
    assert!(log.lock().expect("test log").is_empty());
}

#[test]
fn failed_start_rolls_back_once_in_reverse_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut composition = RuntimeComposition::new();
    composition
        .register(Box::new(TestExtension::ready("a", &[], &log)))
        .expect("register a");
    composition
        .register(Box::new(TestExtension::ready("b", &["a"], &log)))
        .expect("register b");
    let mut failing = TestExtension::ready("c", &["b"], &log);
    failing.start_result = Err("failed".to_owned());
    composition
        .register(Box::new(failing))
        .expect("register failing");

    assert!(matches!(
        composition.start(),
        Err(CompositionError::StartFailed { id, .. }) if id == "c"
    ));
    assert_eq!(
        *log.lock().expect("test log"),
        vec!["start:a", "start:b", "start:c", "stop:b", "stop:a"]
    );
    composition.stop().expect("second stop is idempotent");
}

#[test]
fn teardown_failure_does_not_block_remaining_extensions() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut composition = RuntimeComposition::new();
    composition
        .register(Box::new(TestExtension::ready("a", &[], &log)))
        .expect("register a");
    let mut b = TestExtension::ready("b", &["a"], &log);
    b.stop_fails = true;
    composition.register(Box::new(b)).expect("register b");
    composition
        .register(Box::new(TestExtension::ready("c", &["b"], &log)))
        .expect("register c");

    composition.start().expect("start composition");
    assert!(matches!(
        composition.stop(),
        Err(CompositionError::StopFailed { .. })
    ));
    assert_eq!(
        *log.lock().expect("test log"),
        vec![
            "start:a", "start:b", "start:c", "stop:c", "stop:b", "stop:a"
        ]
    );
}
