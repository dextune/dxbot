#![forbid(unsafe_code)]

pub mod harness;
pub mod transport;
pub mod protocol;
pub mod real_provider;
pub mod deepseek;

pub use harness::{
    Evidence, HarnessAdapter, HarnessError, ProviderHost, ProviderInfo, ProviderStatus,
    ReferenceProvider, TaskDescription, TaskResult, TaskStatus,
};
pub use transport::HttpTransport;
pub use protocol::{
    ChatCompletionProtocol, ChatMessage, ProviderError, ProviderEvent, ProviderExecuteConfig,
    ProviderRequest, UsageInfo,
};
pub use real_provider::RealProvider;
pub use deepseek::DeepSeekFlashAdapter;