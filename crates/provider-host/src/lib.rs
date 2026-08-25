#![forbid(unsafe_code)]

pub mod deepseek;
pub mod harness;
pub mod protocol;
pub mod real_provider;
pub mod reference_adapter;
pub mod transport;

pub use deepseek::DeepSeekFlashAdapter;
pub use harness::{
    Evidence, HarnessAdapter, HarnessError, ProviderHost, ProviderInfo, ProviderStatus,
    ReferenceProvider, TaskDescription, TaskResult, TaskStatus,
};
pub use protocol::{
    CancellationToken, ChatCompletionProtocol, ChatMessage, ProviderError, ProviderEvent,
    ProviderExecuteConfig, ProviderRequest, UsageInfo,
};
pub use real_provider::RealProvider;
pub use reference_adapter::{
    ConformanceError, ConformanceOutcome, ConformanceRequest, CredentialMaterial,
    HttpReferenceAdapter, NormalizedUsage, ReferenceAdapter, SubprocessReferenceAdapter,
    UsageSource,
};
pub use transport::{HttpTransport, TransportBuildError};
