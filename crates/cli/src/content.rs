//! Bounded local content materialization.
//!
//! File paths, stdin handles and source variants are local-only. Before a
//! command can cross the durable Prepared boundary they are converted to a
//! bounded text payload or a validated ArtifactRef, so the Runtime never sees a
//! client-local path or descriptor.

use std::fs::{self, File};
use std::io::{Read, stdin};
use std::path::Path;

use application_contract::CliInput;
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::ContentSource;

pub const MAX_MATERIALIZED_CONTENT_BYTES: u64 = 8 * 1024 * 1024;

pub fn materialize_content(input: &mut CliInput) -> Result<(), DxbotError> {
    let Some(source) = input.content.take() else {
        return Ok(());
    };

    input.content = Some(match source {
        ContentSource::Text { value } => {
            ensure_bound(value.len() as u64, "inline text")?;
            ContentSource::Text { value }
        }
        ContentSource::InputFile { path } => {
            let value = read_bounded_file(Path::new(&path))?;
            ContentSource::Text { value }
        }
        ContentSource::Stdin => {
            let mut bytes = Vec::new();
            stdin()
                .lock()
                .take(MAX_MATERIALIZED_CONTENT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| local_error(format!("cannot read stdin: {error}")))?;
            ensure_bound(bytes.len() as u64, "stdin")?;
            ContentSource::Text {
                value: String::from_utf8(bytes).map_err(|_| {
                    input_error("stdin content is not valid UTF-8 for the P0 text contract")
                })?,
            }
        }
        ContentSource::ArtifactRef {
            artifact_id,
            digest,
        } => {
            if artifact_id.trim().is_empty() || digest.trim().is_empty() {
                return Err(input_error(
                    "artifact reference requires non-empty artifact id and digest",
                ));
            }
            ContentSource::ArtifactRef {
                artifact_id,
                digest,
            }
        }
    });
    Ok(())
}

fn read_bounded_file(path: &Path) -> Result<String, DxbotError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| local_error(format!("cannot stat input file {}: {error}", path.display())))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(input_error(format!(
            "input file must be a direct regular file: {}",
            path.display()
        )));
    }
    ensure_bound(metadata.len(), "input file")?;

    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .and_then(|file| {
            file.take(MAX_MATERIALIZED_CONTENT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map(|_| ())
        })
        .map_err(|error| local_error(format!("cannot read input file {}: {error}", path.display())))?;
    ensure_bound(bytes.len() as u64, "input file")?;
    String::from_utf8(bytes).map_err(|_| {
        input_error(format!(
            "input file is not valid UTF-8 for the P0 text contract: {}",
            path.display()
        ))
    })
}

fn ensure_bound(length: u64, label: &str) -> Result<(), DxbotError> {
    if length > MAX_MATERIALIZED_CONTENT_BYTES {
        return Err(input_error(format!(
            "{label} exceeds {} bytes",
            MAX_MATERIALIZED_CONTENT_BYTES
        )));
    }
    Ok(())
}

fn input_error(message: impl Into<String>) -> DxbotError {
    error(ErrorCode::InvalidInput, ErrorCategory::Input, message)
}

fn local_error(message: impl Into<String>) -> DxbotError {
    error(ErrorCode::StorageOrCorruption, ErrorCategory::Local, message)
}

fn error(
    code: ErrorCode,
    category: ErrorCategory,
    message: impl Into<String>,
) -> DxbotError {
    DxbotError {
        code,
        category,
        message: message.into(),
        retryable: false,
        operation_ref: None,
        target_refs: Vec::new(),
        field_violations: Vec::new(),
        current_revision: None,
        current_generation: None,
        resume_cursor: None,
        next_actions: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn inline_text_is_preserved() {
        let mut input = application_contract::parse_bound_input(&args(&[
            "conversation-send",
            "conversation-a",
            "hello",
        ]))
        .expect("input parses");
        materialize_content(&mut input).expect("text materializes");
        assert_eq!(
            input.content,
            Some(ContentSource::Text {
                value: "hello".to_owned()
            })
        );
    }

    #[test]
    fn oversized_inline_text_is_rejected_before_prepared() {
        let mut input = application_contract::parse_bound_input(&args(&[
            "conversation-send",
            "conversation-a",
            "hello",
        ]))
        .expect("input parses");
        input.content = Some(ContentSource::Text {
            value: "x".repeat(MAX_MATERIALIZED_CONTENT_BYTES as usize + 1),
        });
        let error = materialize_content(&mut input).expect_err("content must be bounded");
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }
}
