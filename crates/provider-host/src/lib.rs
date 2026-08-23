#![forbid(unsafe_code)]

pub mod harness;

pub use harness::{
    Evidence, HarnessAdapter, HarnessError, ProviderHost, ProviderInfo, ProviderStatus,
    ReferenceProvider, TaskDescription, TaskResult, TaskStatus,
};