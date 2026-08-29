#![forbid(unsafe_code)]
#![allow(clippy::result_large_err)]

pub mod error;
pub mod receipt;
pub mod types;

pub use error::DxbotError;
pub use receipt::{ReceiptDisposition, ReceiptRecord};
pub use types::*;
