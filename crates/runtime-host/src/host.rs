//! Runtime host lifecycle: start/status/graceful stop/explicit host stop
//! with generation fencing.

use runtime_bootstrap::BootstrapEndpoint;

use crate::composition::{CompositionError, RuntimeComposition};

/// Observable lifecycle states of a runtime host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostStatus {
    Stopped,
    Running,
    Degraded,
}

/// Errors produced by host lifecycle commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// `stop_host` generation does not match the host's current generation.
    GenerationFenced { expected: i64, provided: i64 },
    /// Runtime extension composition could not reach or leave readiness.
    Composition(CompositionError),
}

/// The runtime host bound to a single verified endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHost {
    endpoint: BootstrapEndpoint,
    status: HostStatus,
    host_generation: i64,
}

impl RuntimeHost {
    /// Bind a host to `endpoint`. The host starts in [`HostStatus::Stopped`].
    pub fn new(endpoint: BootstrapEndpoint) -> Self {
        Self {
            host_generation: endpoint.host_generation,
            endpoint,
            status: HostStatus::Stopped,
        }
    }

    /// Start a host that has no registered runtime composition.
    pub fn start(&mut self) -> Result<HostStatus, Error> {
        match self.status {
            HostStatus::Running => Ok(HostStatus::Running),
            HostStatus::Stopped | HostStatus::Degraded => {
                if !self.endpoint.verified {
                    self.status = HostStatus::Degraded;
                    return Ok(HostStatus::Degraded);
                }
                self.status = HostStatus::Running;
                Ok(HostStatus::Running)
            }
        }
    }

    /// Start a host with a validated runtime composition. The host is not
    /// published as `Running` until every mandatory extension is ready.
    pub fn start_composed(
        &mut self,
        composition: &mut RuntimeComposition,
    ) -> Result<HostStatus, Error> {
        if self.status == HostStatus::Running {
            return Ok(HostStatus::Running);
        }
        if !self.endpoint.verified {
            self.status = HostStatus::Degraded;
            return Ok(HostStatus::Degraded);
        }

        match composition.start() {
            Ok(readiness) if readiness.mandatory_ready_count == readiness.mandatory_total => {
                self.status = HostStatus::Running;
                Ok(HostStatus::Running)
            }
            Ok(_) => {
                self.status = HostStatus::Degraded;
                Ok(HostStatus::Degraded)
            }
            Err(error) => {
                self.status = HostStatus::Degraded;
                Err(Error::Composition(error))
            }
        }
    }

    /// The current lifecycle status.
    pub fn status(&self) -> HostStatus {
        self.status
    }

    /// Gracefully drain and stop a host that has no registered composition.
    pub fn stop_graceful(&mut self) -> Result<HostStatus, Error> {
        if self.status == HostStatus::Stopped {
            return Ok(HostStatus::Stopped);
        }
        self.status = HostStatus::Stopped;
        Ok(HostStatus::Stopped)
    }

    /// Drain composed extensions in reverse dependency order. A component
    /// teardown failure does not keep the host observable as running.
    pub fn stop_composed(
        &mut self,
        composition: &mut RuntimeComposition,
    ) -> Result<HostStatus, Error> {
        let result = composition.stop();
        self.status = HostStatus::Stopped;
        result.map(|()| HostStatus::Stopped).map_err(Error::Composition)
    }

    /// Explicitly stop the host, but only if `host_generation` matches the
    /// host's current generation. A mismatched generation is fenced and the
    /// host keeps running.
    pub fn stop_host(&mut self, host_generation: i64) -> Result<HostStatus, Error> {
        if host_generation != self.host_generation {
            return Err(Error::GenerationFenced {
                expected: self.host_generation,
                provided: host_generation,
            });
        }
        self.stop_graceful()
    }
}
