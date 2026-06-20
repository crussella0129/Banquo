pub mod windows;

use crate::config::ShellProfile;
use std::path::PathBuf;

pub fn apply_window_effects(config: &crate::config::BanquoConfig, frame: &mut eframe::Frame) {
    #[cfg(target_os = "windows")]
    windows::apply_effects(config, frame);
    // Window effects are Windows-only for now; the args are unused elsewhere.
    #[cfg(not(target_os = "windows"))]
    let _ = (config, frame);
}

/// Ensure the visible Banquo runs **outside** the launching terminal's job, so
/// closing that terminal can't take it down (the residual case after the
/// sprint-11 GUI-subsystem change).
///
/// Windows + release only: re-spawns Banquo detached and broken away from any
/// breakable job (via the **safe** `CommandExt::creation_flags` — no `unsafe`,
/// ADR-002 intact), then exits the original. A `BANQUO_DETACHED` sentinel stops
/// the relaunched child from recursing. No-op off Windows and in debug builds
/// (debug `cargo run` is the dev loop and *should* stay shell-attached).
///
/// Honest limit: a `KILL_ON_JOB_CLOSE` job that forbids breakaway cannot be
/// escaped in-process by any code; there `spawn()` fails and we run in place
/// (never worse than today) — launch from the Start-menu shortcut to avoid it.
pub fn ensure_detached() {
    #[cfg(all(windows, not(debug_assertions)))]
    win_detach::run();
}

// Present only where its contents are actually used — release (the real path)
// or `cargo test` (the unit tests). Absent from the windows-debug bin and off
// Windows, so there is no dead code under `-D warnings` (see the four-target
// walk in sprints/s12 build-plan).
#[cfg(all(windows, any(not(debug_assertions), test)))]
mod win_detach {
    use std::ffi::{OsStr, OsString};
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::Command;

    /// Env sentinel marking the already-detached child so it never re-detaches.
    const SENTINEL: &str = "BANQUO_DETACHED";
    /// `CREATE_BREAKAWAY_FROM_JOB` — leave the parent's job object (if allowed).
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    /// `DETACHED_PROCESS` — start with no inherited console (we're GUI-subsystem;
    /// ConPTY later allocates its own pseudoconsoles per shell).
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    /// We should relaunch detached iff the sentinel env var is **absent** (i.e.
    /// we are the original, not the already-detached child). Takes the env value
    /// directly so the real recursion-guard decision is unit-testable without
    /// touching process state.
    pub(super) fn should_detach(sentinel: Option<&OsStr>) -> bool {
        sentinel.is_none()
    }

    /// Build (no side effects) the command that relaunches this exe detached and
    /// broken away from the job, carrying the sentinel + forwarded args.
    pub(super) fn build_relaunch_command(exe: &Path, args: &[OsString]) -> Command {
        let mut cmd = Command::new(exe);
        cmd.args(args);
        cmd.env(SENTINEL, "1");
        cmd.creation_flags(CREATE_BREAKAWAY_FROM_JOB | DETACHED_PROCESS);
        cmd
    }

    /// The effecting guard (release-only — its sole caller is the release-gated
    /// `ensure_detached`, so it is absent from the test target).
    #[cfg(not(debug_assertions))]
    pub(super) fn run() {
        // Already detached? The child short-circuits before any spawn.
        if !should_detach(std::env::var_os(SENTINEL).as_deref()) {
            return;
        }
        let exe = match std::env::current_exe() {
            Ok(e) => e,
            Err(_) => return, // can't relaunch — run in place
        };
        let args: Vec<OsString> = std::env::args_os().skip(1).collect();
        // On success the detached child owns the only window; exit the original.
        // On failure (breakaway denied) fall through and run in place.
        if build_relaunch_command(&exe, &args).spawn().is_ok() {
            std::process::exit(0);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_should_detach_true_when_no_sentinel() {
            // Original process (no sentinel) → must relaunch detached.
            assert!(should_detach(None));
        }

        #[test]
        fn test_should_detach_false_when_sentinel_set() {
            // Already-detached child (sentinel present) → must NOT re-detach.
            assert!(!should_detach(Some(OsStr::new("1"))));
        }

        #[test]
        fn test_build_relaunch_command_program_and_args() {
            let exe = Path::new("C:/b/banquo.exe");
            let args = vec![OsString::from("--foo"), OsString::from("bar")];
            let cmd = build_relaunch_command(exe, &args);
            assert_eq!(cmd.get_program(), exe.as_os_str());
            let got: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
            assert_eq!(got, vec![OsString::from("--foo"), OsString::from("bar")]);
        }

        #[test]
        fn test_build_relaunch_command_sets_sentinel_env() {
            let cmd = build_relaunch_command(Path::new("banquo.exe"), &[]);
            let has_sentinel = cmd
                .get_envs()
                .any(|(k, v)| k == "BANQUO_DETACHED" && v == Some(OsString::from("1").as_os_str()));
            assert!(has_sentinel, "relaunch command must set BANQUO_DETACHED=1");
        }
    }
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
