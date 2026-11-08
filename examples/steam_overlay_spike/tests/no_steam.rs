//! T217.3 `test_spike_runs_without_steam`: the spike runs, presents and exits
//! cleanly without Steam, and reports `steam: off`.
//!
//! Runs the real binary with `--no-steam --run-seconds 1 --renderer auto`, so
//! the pre-device hook skips `Client::init_app` and the presenter still builds
//! and presents. It opens a window for about a second and needs a GPU adapter.
//! On a runner with no adapter or display, set `BANQUO_ALLOW_HEADLESS=1` to
//! report a skip (with the reason) instead of a failure, as
//! `banquo-presenter`'s `host_owned_loop` test does.

use std::process::Command;

const ALLOW_HEADLESS: &str = "BANQUO_ALLOW_HEADLESS";

fn headless_allowed() -> bool {
    std::env::var(ALLOW_HEADLESS)
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

#[test]
fn test_spike_runs_without_steam() {
    let exe = env!("CARGO_BIN_EXE_steam_overlay_spike");
    let out = Command::new(exe)
        .args(["--no-steam", "--run-seconds", "1", "--renderer", "auto"])
        .output()
        .unwrap_or_else(|e| panic!("could not start {exe}: {e}"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let report = format!(
        "status {:?}\n--- stdout\n{stdout}--- stderr\n{stderr}",
        out.status
    );

    let no_gpu = stdout.contains("error: no usable wgpu backend")
        || stderr.contains("cannot create event loop")
        || stdout.contains("error: window creation failed");
    if no_gpu && headless_allowed() {
        println!("test_spike_runs_without_steam skipped: no GPU adapter or display ({ALLOW_HEADLESS} set)\n{report}");
        return;
    }

    assert!(
        out.status.success(),
        "spike did not exit cleanly without Steam\n{report}"
    );
    // The hook ran and turned Steam off before the device existed.
    assert!(
        stdout.contains("[spike] steam: off (--no-steam)"),
        "no `steam: off` from the pre-device hook\n{report}"
    );
    // The presenter was built and frames ran until the time limit, not an early exit.
    assert!(
        stdout.contains("[spike] backend="),
        "presenter was not built\n{report}"
    );
    assert!(
        stdout.contains("[spike] --run-seconds 1 reached"),
        "spike exited before --run-seconds 1 was reached\n{report}"
    );
    // The exit summary carries the same label the HUD draws (`steam_label`).
    let exit_line = stdout
        .lines()
        .find(|l| l.starts_with("[spike] exit: "))
        .unwrap_or_else(|| panic!("no exit summary line\n{report}"));
    assert!(
        exit_line.ends_with(" steam: off"),
        "exit summary should end with the HUD label `steam: off`: {exit_line}\n{report}"
    );
    let frames: u64 = exit_line
        .split_whitespace()
        .find_map(|w| w.strip_prefix("frames="))
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no frames= in the exit summary: {exit_line}"));
    assert!(frames > 0, "no frame was presented: {exit_line}\n{report}");
}
