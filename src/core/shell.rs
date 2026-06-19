//! Shell resolution — pure mapping from configured profiles to a spawnable spec.
//!
//! A *shell* is just the child program the PTY runs. This module turns a
//! `ShellConfig` plus an optional requested name into a [`ResolvedShell`]
//! (defined in [`super::pty`]) — or `None`, which signals the caller to fall
//! back to the OS default program (`new_default_prog`), preserving the
//! pre-sprint-11 behavior exactly. Everything here is pure and side-effect-free
//! so it can be unit-tested without spawning a process.

use crate::config::{BanquoConfig, ShellProfile};
use crate::core::pty::ResolvedShell;

/// Convert a configured [`ShellProfile`] into a spawnable [`ResolvedShell`].
pub fn profile_to_resolved(p: &ShellProfile) -> ResolvedShell {
    ResolvedShell {
        prog: p.command.clone(),
        args: p.args.clone(),
        cwd: p.cwd.clone(),
        env: p
            .env
            .as_ref()
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default(),
    }
}

/// Resolve which shell to launch.
///
/// - `Some(name)` selects the profile with that `name`.
/// - `None` selects the profile named by `config.shell.default`.
///
/// Returns `None` when no matching profile exists (including: an unknown
/// requested name, or no configured default). `None` means "use the OS default
/// program" for the startup path, and "do nothing" for an explicit palette
/// request — the caller decides.
pub fn resolve_shell(config: &BanquoConfig, name: Option<&str>) -> Option<ResolvedShell> {
    let target = match name {
        Some(n) => n,
        None => config.shell.default.as_deref()?,
    };
    config
        .shell
        .profiles
        .iter()
        .find(|p| p.name == target)
        .map(profile_to_resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ShellConfig;
    use std::collections::BTreeMap;

    fn cfg_with(default: Option<&str>, profiles: Vec<ShellProfile>) -> BanquoConfig {
        BanquoConfig {
            shell: ShellConfig {
                default: default.map(|s| s.to_string()),
                profiles,
            },
            ..Default::default()
        }
    }

    fn profile(name: &str, command: &str) -> ShellProfile {
        ShellProfile {
            name: name.to_string(),
            command: command.to_string(),
            args: vec![],
            cwd: None,
            env: None,
        }
    }

    #[test]
    fn test_resolve_named_profile() {
        let cfg = cfg_with(None, vec![profile("pwsh", "pwsh.exe")]);
        let resolved = resolve_shell(&cfg, Some("pwsh")).expect("profile found");
        assert_eq!(resolved.prog, "pwsh.exe");
    }

    #[test]
    fn test_resolve_default_when_name_none() {
        let cfg = cfg_with(
            Some("cmd"),
            vec![profile("pwsh", "pwsh.exe"), profile("cmd", "cmd.exe")],
        );
        let resolved = resolve_shell(&cfg, None).expect("default resolves");
        assert_eq!(resolved.prog, "cmd.exe");
    }

    #[test]
    fn test_resolve_falls_back_to_none() {
        // No default, and an unknown requested name → None (use new_default_prog).
        let cfg = cfg_with(None, vec![profile("pwsh", "pwsh.exe")]);
        assert!(resolve_shell(&cfg, None).is_none());
        assert!(resolve_shell(&cfg, Some("zsh")).is_none());
    }

    #[test]
    fn test_resolve_maps_args_cwd_env() {
        let mut env = BTreeMap::new();
        env.insert("FOO".to_string(), "bar".to_string());
        let p = ShellProfile {
            name: "wsl".to_string(),
            command: "wsl.exe".to_string(),
            args: vec!["-d".to_string(), "Ubuntu".to_string()],
            cwd: Some("/home".to_string()),
            env: Some(env),
        };
        let cfg = cfg_with(None, vec![p]);
        let r = resolve_shell(&cfg, Some("wsl")).unwrap();
        assert_eq!(r.args, vec!["-d".to_string(), "Ubuntu".to_string()]);
        assert_eq!(r.cwd.as_deref(), Some("/home"));
        assert_eq!(r.env, vec![("FOO".to_string(), "bar".to_string())]);
    }
}
