//! Production execution-context projection for CLI input.
//!
//! Parser-side `CliInput::instance_id` / `principal_ref` remain compatibility
//! conveniences for isolated projection tests. Production orchestration must
//! bind a verified Runtime Instance and a principal learned from the trusted
//! local transport boundary before the payload becomes dispatchable.

use dxbot_core::DxbotError;
use dxbot_core::types::{CommandPayload, InstanceId, PrincipalRef};

use crate::{CliInput, contract};

/// Identity material supplied by verified discovery/authenticated transport.
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

/// Project a command payload using only trusted execution identity.
///
/// Local `--profile` / `--instance` values may participate in discovery before
/// this function is called, but they cannot become authenticated identity here.
pub fn project_for_execution(
    input: &CliInput,
    context: &ExecutionContext,
) -> Result<CommandPayload, DxbotError> {
    // Normalize the compatibility projection to the already-verified Instance
    // so target materialization cannot fall back to profile/local synthetic IDs.
    let mut normalized = input.clone();
    normalized.global_options.instance = Some(context.instance_id.0.clone());
    normalized.global_options.profile = None;

    let mut payload = contract::CommandPayload::from_cli_input(&normalized)?.inner;
    payload.instance_id = context.instance_id.clone();
    payload.principal_ref = context.principal_ref.clone();

    // Instance-targeted operations must bind exactly to the verified Instance.
    if matches!(
        &payload.canonical_target,
        dxbot_core::types::CanonicalTarget::Instance(_)
    ) {
        payload.canonical_target =
            dxbot_core::types::CanonicalTarget::Instance(context.instance_id.clone());
    }
    Ok(payload)
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
}
