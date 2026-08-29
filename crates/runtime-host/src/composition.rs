//! Deterministic runtime extension composition owned by `runtime-host`.
//!
//! The graph descriptor remains private so this module does not become a
//! second plugin/configuration schema.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use runtime_audit::{
    DiagnosticComponent, DiagnosticFamily, DiagnosticReason, DiagnosticSeverity, DiagnosticSink,
    SafeAttribute, SafeAttributeKey,
};

const MAX_SUMMARY_ITEMS: usize = 16;
const MAX_EXTENSION_ID_BYTES: usize = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionKind {
    CommonCapability,
    ProviderAdapter,
    InternalService,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionReadiness {
    Ready,
    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCompositionState {
    Declared,
    Validated,
    Starting,
    Ready,
    Draining,
    Stopped,
    FailedStop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeReadiness {
    pub state: RuntimeCompositionState,
    pub mandatory_ready_count: usize,
    pub mandatory_total: usize,
    pub degraded_extension_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceHealth {
    pub readiness: RuntimeReadiness,
    pub diagnostic_watermark: u64,
    pub source_watermark: i64,
    pub latest_backstop_verified: bool,
}

pub trait RuntimeExtension: fmt::Debug + Send {
    fn id(&self) -> &str;
    fn kind(&self) -> ExtensionKind;
    fn dependencies(&self) -> &[String];
    fn required(&self) -> bool {
        true
    }
    fn compatibility_generation(&self) -> i64 {
        1
    }
    fn start(&mut self) -> Result<ExtensionReadiness, String>;
    fn stop(&mut self) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionError {
    RegistrationClosed {
        state: RuntimeCompositionState,
    },
    InvalidExtensionId {
        id: String,
    },
    InvalidCompatibilityGeneration {
        id: String,
        generation: i64,
    },
    DuplicateExtension {
        id: String,
    },
    MissingDependency {
        id: String,
        dependency: String,
    },
    SelfDependency {
        id: String,
    },
    DependencyCycle {
        extension_ids: Vec<String>,
    },
    StartFailed {
        id: String,
        rollback_failures: Vec<String>,
    },
    MandatoryExtensionDegraded {
        id: String,
        rollback_failures: Vec<String>,
    },
    StopFailed {
        extension_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExtensionDescriptor {
    id: String,
    kind: ExtensionKind,
    dependencies: Vec<String>,
    required: bool,
    compatibility_generation: i64,
}

struct ExtensionEntry {
    descriptor: ExtensionDescriptor,
    extension: Box<dyn RuntimeExtension>,
    readiness: Option<ExtensionReadiness>,
}

impl fmt::Debug for ExtensionEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExtensionEntry")
            .field("descriptor", &self.descriptor)
            .field("readiness", &self.readiness)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct RuntimeComposition {
    entries: Vec<ExtensionEntry>,
    resolved_order: Vec<usize>,
    started_order: Vec<usize>,
    state: RuntimeCompositionState,
    diagnostics: Option<DiagnosticSink>,
}

impl Default for RuntimeComposition {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeComposition {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            resolved_order: Vec::new(),
            started_order: Vec::new(),
            state: RuntimeCompositionState::Declared,
            diagnostics: None,
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: DiagnosticSink) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    pub fn register(
        &mut self,
        extension: Box<dyn RuntimeExtension>,
    ) -> Result<(), CompositionError> {
        if !matches!(
            self.state,
            RuntimeCompositionState::Declared | RuntimeCompositionState::Validated
        ) || !self.started_order.is_empty()
        {
            return Err(CompositionError::RegistrationClosed { state: self.state });
        }
        let descriptor = ExtensionDescriptor {
            id: extension.id().to_owned(),
            kind: extension.kind(),
            dependencies: extension.dependencies().to_vec(),
            required: extension.required(),
            compatibility_generation: extension.compatibility_generation(),
        };
        self.entries.push(ExtensionEntry {
            descriptor,
            extension,
            readiness: None,
        });
        self.resolved_order.clear();
        self.state = RuntimeCompositionState::Declared;
        Ok(())
    }

    pub fn resolve(&mut self) -> Result<Vec<String>, CompositionError> {
        let mut by_id = BTreeMap::new();
        for (index, entry) in self.entries.iter().enumerate() {
            validate_descriptor(&entry.descriptor)?;
            if by_id.insert(entry.descriptor.id.clone(), index).is_some() {
                return Err(CompositionError::DuplicateExtension {
                    id: entry.descriptor.id.clone(),
                });
            }
        }

        let mut indegree = vec![0_usize; self.entries.len()];
        let mut dependents = vec![Vec::new(); self.entries.len()];
        for (index, entry) in self.entries.iter().enumerate() {
            let mut unique = BTreeSet::new();
            for dependency in &entry.descriptor.dependencies {
                if dependency == &entry.descriptor.id {
                    return Err(CompositionError::SelfDependency {
                        id: entry.descriptor.id.clone(),
                    });
                }
                if !unique.insert(dependency) {
                    continue;
                }
                let Some(dependency_index) = by_id.get(dependency).copied() else {
                    return Err(CompositionError::MissingDependency {
                        id: entry.descriptor.id.clone(),
                        dependency: dependency.clone(),
                    });
                };
                indegree[index] = indegree[index].saturating_add(1);
                dependents[dependency_index].push(index);
            }
        }

        let mut ready = BTreeSet::new();
        for (index, degree) in indegree.iter().copied().enumerate() {
            if degree == 0 {
                ready.insert((self.entries[index].descriptor.id.clone(), index));
            }
        }
        let mut order = Vec::with_capacity(self.entries.len());
        while let Some((_, index)) = ready.pop_first() {
            order.push(index);
            for dependent in &dependents[index] {
                indegree[*dependent] = indegree[*dependent].saturating_sub(1);
                if indegree[*dependent] == 0 {
                    ready.insert((self.entries[*dependent].descriptor.id.clone(), *dependent));
                }
            }
        }
        if order.len() != self.entries.len() {
            let mut extension_ids = self
                .entries
                .iter()
                .enumerate()
                .filter(|(index, _)| indegree[*index] > 0)
                .map(|(_, entry)| entry.descriptor.id.clone())
                .collect::<Vec<_>>();
            extension_ids.sort();
            extension_ids.truncate(MAX_SUMMARY_ITEMS);
            return Err(CompositionError::DependencyCycle { extension_ids });
        }

        self.resolved_order = order;
        self.state = RuntimeCompositionState::Validated;
        Ok(self.resolved_ids())
    }

    pub fn start(&mut self) -> Result<RuntimeReadiness, CompositionError> {
        if self.state == RuntimeCompositionState::Ready {
            return Ok(self.readiness());
        }
        if self.resolved_order.len() != self.entries.len() {
            if let Err(error) = self.resolve() {
                self.emit_diagnostic(
                    DiagnosticFamily::RuntimeExtensionGraph,
                    DiagnosticReason::GraphInvalid,
                    DiagnosticSeverity::Error,
                    None,
                );
                return Err(error);
            }
        }

        self.started_order.clear();
        for entry in &mut self.entries {
            entry.readiness = None;
        }
        self.state = RuntimeCompositionState::Starting;
        for index in self.resolved_order.clone() {
            let id = self.entries[index].descriptor.id.clone();
            match self.entries[index].extension.start() {
                Ok(readiness) => {
                    self.entries[index].readiness = Some(readiness);
                    self.started_order.push(index);
                    if self.entries[index].descriptor.required
                        && readiness != ExtensionReadiness::Ready
                    {
                        let rollback_failures = self.rollback_started();
                        self.state = RuntimeCompositionState::Declared;
                        self.emit_start_failure(&id);
                        return Err(CompositionError::MandatoryExtensionDegraded {
                            id,
                            rollback_failures,
                        });
                    }
                }
                Err(_) => {
                    let rollback_failures = self.rollback_started();
                    self.state = RuntimeCompositionState::Declared;
                    self.emit_start_failure(&id);
                    return Err(CompositionError::StartFailed {
                        id,
                        rollback_failures,
                    });
                }
            }
        }
        self.state = RuntimeCompositionState::Ready;
        Ok(self.readiness())
    }

    pub fn stop(&mut self) -> Result<(), CompositionError> {
        if self.started_order.is_empty() {
            self.state = RuntimeCompositionState::Stopped;
            return Ok(());
        }
        self.state = RuntimeCompositionState::Draining;
        let failures = self.rollback_started();
        if failures.is_empty() {
            self.state = RuntimeCompositionState::Stopped;
            Ok(())
        } else {
            self.state = RuntimeCompositionState::FailedStop;
            self.emit_diagnostic(
                DiagnosticFamily::RuntimeLifecycle,
                DiagnosticReason::StopFailed,
                DiagnosticSeverity::Error,
                failures.first().map(String::as_str),
            );
            Err(CompositionError::StopFailed {
                extension_ids: failures,
            })
        }
    }

    pub fn readiness(&self) -> RuntimeReadiness {
        let mandatory_total = self
            .entries
            .iter()
            .filter(|entry| entry.descriptor.required)
            .count();
        let mandatory_ready_count = self
            .entries
            .iter()
            .filter(|entry| {
                entry.descriptor.required && entry.readiness == Some(ExtensionReadiness::Ready)
            })
            .count();
        let mut degraded_extension_ids = self
            .entries
            .iter()
            .filter(|entry| entry.readiness == Some(ExtensionReadiness::Degraded))
            .map(|entry| entry.descriptor.id.clone())
            .collect::<Vec<_>>();
        degraded_extension_ids.sort();
        degraded_extension_ids.truncate(MAX_SUMMARY_ITEMS);
        RuntimeReadiness {
            state: self.state,
            mandatory_ready_count,
            mandatory_total,
            degraded_extension_ids,
        }
    }

    pub fn has_kind(&self, kind: ExtensionKind) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.descriptor.kind == kind)
    }

    pub fn health(&self, source_watermark: i64, latest_backstop_verified: bool) -> InstanceHealth {
        let diagnostic_watermark = self
            .diagnostics
            .as_ref()
            .and_then(|sink| sink.latest_watermark().ok())
            .unwrap_or(0);
        InstanceHealth {
            readiness: self.readiness(),
            diagnostic_watermark,
            source_watermark,
            latest_backstop_verified,
        }
    }

    pub fn resolved_ids(&self) -> Vec<String> {
        self.resolved_order
            .iter()
            .map(|index| self.entries[*index].descriptor.id.clone())
            .collect()
    }

    fn emit_start_failure(&self, id: &str) {
        self.emit_diagnostic(
            DiagnosticFamily::RuntimeLifecycle,
            DiagnosticReason::StartFailed,
            DiagnosticSeverity::Error,
            Some(id),
        );
    }

    fn emit_diagnostic(
        &self,
        family: DiagnosticFamily,
        reason: DiagnosticReason,
        severity: DiagnosticSeverity,
        extension_id: Option<&str>,
    ) {
        let Some(sink) = &self.diagnostics else {
            return;
        };
        let attributes = extension_id
            .map(|id| vec![SafeAttribute::new(SafeAttributeKey::ExtensionId, id)])
            .unwrap_or_default();
        let _ = sink.emit(
            DiagnosticComponent::RuntimeHost,
            "runtime-composition",
            family,
            reason,
            severity,
            &attributes,
        );
    }

    fn rollback_started(&mut self) -> Vec<String> {
        let mut failures = Vec::new();
        while let Some(index) = self.started_order.pop() {
            if self.entries[index].extension.stop().is_err() && failures.len() < MAX_SUMMARY_ITEMS {
                failures.push(self.entries[index].descriptor.id.clone());
            }
            self.entries[index].readiness = None;
        }
        failures
    }
}

fn validate_descriptor(descriptor: &ExtensionDescriptor) -> Result<(), CompositionError> {
    if descriptor.id.is_empty()
        || descriptor.id.len() > MAX_EXTENSION_ID_BYTES
        || !descriptor
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(CompositionError::InvalidExtensionId {
            id: descriptor.id.clone(),
        });
    }
    if descriptor.compatibility_generation <= 0 {
        return Err(CompositionError::InvalidCompatibilityGeneration {
            id: descriptor.id.clone(),
            generation: descriptor.compatibility_generation,
        });
    }
    Ok(())
}
