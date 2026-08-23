//! Runtime host lifecycle: start/status/graceful stop/explicit host stop
//! with generation fencing.
//!
//! The host is the execution side of a verified runtime endpoint. Lifecycle
//! state is held in a single explicit [`HostStatus`]; transitions happen only
//! through the documented command methods, never through public field
//! mutation. `stop_host` rejects a stop whose generation does not match the
//! host's current generation so a stale caller cannot stop a newer host.

use runtime_bootstrap::BootstrapEndpoint;

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

    /// Start the host. Returns the resulting status: `Running` for a verified
    /// endpoint, `Degraded` when the endpoint is not verified. Idempotent while
    /// already running.
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

    /// The current lifecycle status.
    pub fn status(&self) -> HostStatus {
        self.status
    }

    /// Gracefully drain and stop the host. Returns `Stopped`. Idempotent from
    /// the stopped state.
    pub fn stop_graceful(&mut self) -> Result<HostStatus, Error> {
        if self.status == HostStatus::Stopped {
            return Ok(HostStatus::Stopped);
        }
        self.status = HostStatus::Stopped;
        Ok(HostStatus::Stopped)
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