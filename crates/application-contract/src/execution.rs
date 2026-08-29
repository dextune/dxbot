//! Production execution-context projection for CLI input.
//!
//! Parser-side `CliInput::instance_id` / `principal_ref` remain compatibility
//! conveniences for isolated projection tests. Production orchestration binds a
//! verified Runtime Instance and authenticated Principal, and removes every
//! registry-owned `@local` field before producing a wire payload.

use dxbot_core::DxbotError;
use dxbot_core::types::{CommandPayload, InstanceId, PrincipalRef};

use crate::{CliInput, contract, metadata_for_key};
use crate::util;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionContext {
    pub instance_id: InstanceId,
    pub principal_ref: PrincipalRef,
}

impl ExecutionContext {
    pub fn new(instance_id: InstanceId, principal_ref: PrincipalRef) -> Self {
        Self {
            instance_id,
            principal_ref,
        }
    }
}

pub fn project_for_execution(
    input: &CliInput,
    context: &ExecutionContext,
) -> Result<CommandPayload, DxbotError> {
    let mut normalized = input.clone();
    normalized.global_options.instance = Some(context.instance_id.0.clone());
    normalized.global_options.profile = None;

    let mut payload = contract::CommandPayload::from_cli_input(&normalized)?.inner;
    payload.instance_id = context.instance_id.clone();
    payload.principal_ref = context.principal_ref.clone();
    remove_local_semantic_fields(&normalized, &mut payload)?;

    if matches!(
        &payload.canonical_target,
        dxbot_core::types::CanonicalTarget::Instance(_)
    ) {
        payload.canonical_target =
            dxbot_core::types::CanonicalTarget::Instance(context.instance_id.clone());
    }
    Ok(payload)
}

/// `typed_fields` is already parsed/validated by `invocation`; this projection
/// only consumes the frozen `@local` markers to prevent validated UX state from
/// becoming semantic wire state. No command-specific local field list exists.
fn remove_local_semantic_fields(
    input: &CliInput,
    payload: &mut CommandPayload,
) -> Result<(), DxbotError> {
    let metadata = metadata_for_key(&input.command_key).ok_or_else(|| {
        util::invariant(format!(
            "known command '{}' has no metadata row",
            input.command_key
        ))
    })?;
    let options = payload.semantic_options.as_object_mut().ok_or_else(|| {
        util::invariant("CommandPayload semantic_options must be a JSON object")
    })?;
    for raw in metadata.typed_fields.split(';') {
        let raw = raw.trim();
        if raw.is_empty() || !raw.contains("@local") {
            continue;
        }
        let Some((name, _)) = raw.split_once(':') else {
            return Err(util::invariant(format!(
                "invalid typed field metadata for {}: {raw}",
                input.command_key
            )));
        };
        options.remove(name.trim());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use dxbot_core::types::CanonicalTarget;

    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn trusted_context_overrides_parser_identity_hints() {
        let input = CliInput::parse(&args(&[
            "bot-create",
            "--profile",
            "attacker-profile",
            "--instance",
            "attacker-instance",
            "--name",
            "alpha",
        ]))
        .expect("input parses");
        let context = ExecutionContext::new(
            InstanceId("verified-instance".to_owned()),
            PrincipalRef("unix-uid:1000".to_owned()),
        );

        let payload = project_for_execution(&input, &context).expect("projection succeeds");
        assert_eq!(payload.instance_id, context.instance_id);
        assert_eq!(payload.principal_ref, context.principal_ref);
        assert_eq!(
            payload.canonical_target,
            CanonicalTarget::Instance(InstanceId("verified-instance".to_owned()))
        );
    }

    #[test]
    fn registry_local_fields_never_cross_wire() {
        let input = crate::parse_bound_input(&args(&["bot-list", "--all"]))
            .expect("local all parses");
        assert_eq!(input.fields["all"], true);
        let context = ExecutionContext::new(
            InstanceId("verified-instance".to_owned()),
            PrincipalRef("local:verified-instance:uid:1000".to_owned()),
        );
        let payload = project_for_execution(&input, &context).expect("projection succeeds");
        assert!(payload.semantic_options.get("all").is_none());
    }
}
