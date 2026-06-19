pub mod windows;

use crate::config::ShellProfile;
use std::path::PathBuf;

pub fn apply_window_effects(config: &crate::config::BanquoConfig, frame: &mut eframe::Frame) {
    #[cfg(target_os = "windows")]
    windows::apply_effects(config, frame);
    // Ignore on other OSes for now
}

/// A shell Banquo knows how to look for on `PATH`.
struct ShellCandidate {
    /// Profile name surfaced to the user.
    name: &'static str,
    /// Program to launch (`argv[0]`).
    command: &'static str,
    /// Executable filename to probe for on `PATH`.
    exe: &'static str,
}

/// The shells we probe for, per platform. A *shell* is just a child program,
/// so "detecting" one is purely checking whether its executable is on `PATH`.
#[cfg(windows)]
fn candidates() -> &'static [ShellCandidate] {
    &[
        ShellCandidate {
            name: "pwsh",
            command: "pwsh.exe",
            exe: "pwsh.exe",
        },
        ShellCandidate {
            name: "powershell",
            command: "powershell.exe",
            exe: "powershell.exe",
        },
        ShellCandidate {
            name: "cmd",
            command: "cmd.exe",
            exe: "cmd.exe",
        },
        ShellCandidate {
            name: "bash",
            command: "bash.exe",
            exe: "bash.exe",
        },
        // WSL is detected as a single profile; per-distro enumeration is deferred
        // (avoids the `wsl.exe -l -q` UTF-16LE parsing hazard — we never spawn here).
        ShellCandidate {
            name: "wsl",
            command: "wsl.exe",
            exe: "wsl.exe",
        },
    ]
}

#[cfg(not(windows))]
fn candidates() -> &'static [ShellCandidate] {
    &[
        ShellCandidate {
            name: "bash",
            command: "bash",
            exe: "bash",
        },
        ShellCandidate {
            name: "zsh",
            command: "zsh",
            exe: "zsh",
        },
        ShellCandidate {
            name: "sh",
            command: "sh",
            exe: "sh",
        },
    ]
}

/// The guaranteed fallback profile, so the detected list is never empty.
fn fallback_profile() -> ShellProfile {
    #[cfg(windows)]
    let (name, command) = ("cmd", "cmd.exe");
    #[cfg(not(windows))]
    let (name, command) = ("sh", "/bin/sh");
    ShellProfile {
        name: name.to_string(),
        command: command.to_string(),
        args: vec![],
        cwd: None,
        env: None,
    }
}

fn candidate_to_profile(c: &ShellCandidate) -> ShellProfile {
    ShellProfile {
        name: c.name.to_string(),
        command: c.command.to_string(),
        args: vec![],
        cwd: None,
        env: None,
    }
}

/// Pure detection: which known shells have their executable in `paths`?
///
/// Takes an explicit path list (no ambient state) so it is unit-testable.
/// Always returns a non-empty list — falls back to [`fallback_profile`] when
/// nothing is found.
pub fn detect_in(paths: &[PathBuf]) -> Vec<ShellProfile> {
    let mut found: Vec<ShellProfile> = candidates()
        .iter()
        .filter(|c| paths.iter().any(|dir| dir.join(c.exe).exists()))
        .map(candidate_to_profile)
        .collect();
    if found.is_empty() {
        found.push(fallback_profile());
    }
    found
}

/// Probe the ambient `PATH` for known shells.
pub fn detect_shells() -> Vec<ShellProfile> {
    let paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    detect_in(&paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_shells_never_empty() {
        // Regardless of environment, the detected list always has the fallback.
        assert!(!detect_shells().is_empty());
        // And detect_in with no paths still yields the guaranteed fallback.
        assert!(!detect_in(&[]).is_empty());
    }

    #[test]
    fn test_detect_in_finds_path_entries() {
        // Pick an exe that maps to a candidate on THIS platform.
        #[cfg(windows)]
        let (exe, expected) = ("pwsh.exe", "pwsh");
        #[cfg(not(windows))]
        let (exe, expected) = ("bash", "bash");

        let dir = std::env::temp_dir().join(format!("banquo_detect_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(exe), b"").unwrap();

        let result = detect_in(std::slice::from_ref(&dir));
        let names: Vec<String> = result.iter().map(|p| p.name.clone()).collect();
        // Clean up before asserting so a failure doesn't leak the temp dir.
        std::fs::remove_dir_all(&dir).ok();
        assert!(
            names.iter().any(|n| n == expected),
            "expected `{expected}` in {names:?}"
        );
    }
}
