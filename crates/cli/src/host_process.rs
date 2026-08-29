//! Hidden process entrypoint that delegates Runtime ownership to `runtime-host`.
//!
//! This module contains no Runtime semantics. It only parses the private spawn
//! arguments emitted by `runner::runtime start`, constructs the canonical
//! `LocalRuntimeHost`, and blocks in its serve loop.

use std::ffi::OsString;
use std::path::PathBuf;

pub fn run_runtime_host_process(args: &[OsString]) -> Result<(), String> {
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("P0 local Runtime Host requires Unix".to_owned())
    }
    #[cfg(unix)]
    {
        let parsed = HostArgs::parse(args)?;
        let host = runtime_host::LocalRuntimeHost::start(
            parsed.runtime_root,
            parsed.discovery_root,
            parsed.profile,
        )
        .map_err(|error| format!("cannot start local Runtime Host: {error}"))?;
        host.serve()
            .map_err(|error| format!("local Runtime Host serve loop failed: {error}"))
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct HostArgs {
    runtime_root: PathBuf,
    discovery_root: PathBuf,
    profile: Option<String>,
}

#[cfg(unix)]
impl HostArgs {
    fn parse(args: &[OsString]) -> Result<Self, String> {
        let mut runtime_root = None;
        let mut discovery_root = None;
        let mut profile = None;
        let mut index = 0usize;
        while index < args.len() {
            let key = args[index]
                .to_str()
                .ok_or_else(|| "Runtime Host option name is not valid UTF-8".to_owned())?;
            match key {
                "--runtime-root" => {
                    runtime_root = Some(PathBuf::from(require_value(args, index, key)?));
                    index += 2;
                }
                "--discovery-root" => {
                    discovery_root = Some(PathBuf::from(require_value(args, index, key)?));
                    index += 2;
                }
                "--profile" => {
                    let value = require_value(args, index, key)?
                        .to_str()
                        .ok_or_else(|| "Runtime Host profile is not valid UTF-8".to_owned())?;
                    profile = Some(value.to_owned());
                    index += 2;
                }
                other => return Err(format!("unknown Runtime Host option: {other}")),
            }
        }
        Ok(Self {
            runtime_root: runtime_root
                .ok_or_else(|| "Runtime Host requires --runtime-root".to_owned())?,
            discovery_root: discovery_root
                .ok_or_else(|| "Runtime Host requires --discovery-root".to_owned())?,
            profile,
        })
    }
}

#[cfg(unix)]
fn require_value<'a>(args: &'a [OsString], index: usize, key: &str) -> Result<&'a OsString, String> {
    args.get(index + 1)
        .ok_or_else(|| format!("Runtime Host option {key} requires a value"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    #[cfg(unix)]
    use super::*;

    #[cfg(unix)]
    #[test]
    fn hidden_host_args_are_exact_and_bounded_by_known_options() {
        let args = vec![
            OsString::from("--runtime-root"),
            OsString::from("/tmp/runtime"),
            OsString::from("--discovery-root"),
            OsString::from("/tmp/discovery"),
            OsString::from("--profile"),
            OsString::from("default"),
        ];
        let parsed = HostArgs::parse(&args).expect("internal args parse");
        assert_eq!(parsed.runtime_root, PathBuf::from("/tmp/runtime"));
        assert_eq!(parsed.discovery_root, PathBuf::from("/tmp/discovery"));
        assert_eq!(parsed.profile.as_deref(), Some("default"));
    }
}
