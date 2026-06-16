//! Sprint 0 integration gates (design guarantees + plan EARS, file-level).
//!
//! These assert repository invariants that the unit tests can't see from inside
//! the crate: the manifest's shape, the founding ADRs, and the no-unsafe header.
//! They read source files relative to `CARGO_MANIFEST_DIR`, so they run headless
//! with no window or wgpu context.

use std::fs;
use std::path::Path;

fn repo_file(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// T-001 EARS: package `banquo`, edition 2021, eframe with the `wgpu` feature.
#[test]
fn test_manifest_metadata() {
    let toml = repo_file("Cargo.toml");
    assert!(
        toml.contains("name = \"banquo\""),
        "Cargo.toml must declare package name `banquo`"
    );
    assert!(
        toml.contains("edition = \"2021\""),
        "Cargo.toml must declare edition 2021"
    );
    assert!(toml.contains("eframe"), "Cargo.toml must depend on eframe");
    // Tightened: assert `"wgpu"` lives inside *eframe's* `features` array, not
    // merely somewhere in the file (a stray match in a comment or another dep
    // would otherwise pass while the renderer feature was dropped). Slice from
    // the `eframe` dependency to the close of its inline table.
    let eframe_start = toml.find("eframe =").expect("eframe dependency line");
    let eframe_block = &toml[eframe_start..];
    let block_end = eframe_block.find('}').unwrap_or(eframe_block.len());
    let eframe_block = &eframe_block[..block_end];
    assert!(
        eframe_block.contains("features") && eframe_block.contains("\"wgpu\""),
        "eframe must enable the `wgpu` feature in its own features array \
         (guards against a silently-dropped renderer): {eframe_block}"
    );
}

/// T-002 EARS: the window is transparency-capable and frameless. Mirrors the
/// `forbid(unsafe_code)` source gate — the user's headline instruction
/// ("frameless transparent window") deserves the same cheap text insurance as the
/// no-unsafe header, so an accidental deletion is caught even on a platform that
/// happens to render opaque anyway.
#[test]
fn test_window_is_transparent_and_frameless() {
    let main = repo_file("src/main.rs");
    assert!(
        main.contains("with_transparent(true)"),
        "main.rs must request a transparent viewport"
    );
    assert!(
        main.contains("with_decorations(native_decorations)"),
        "main.rs must request conditional decorations via native_decorations"
    );
}

/// T-006 EARS: decisions.md contains the four founding ADRs by subject.
#[test]
fn test_decisions_has_four_adrs() {
    let decisions = repo_file("decisions.md");
    let adr_count = decisions.matches("ADR-00").count();
    assert!(
        adr_count >= 4,
        "expected at least 4 ADR entries, found {adr_count}"
    );
    for subject in [
        "Crate stack",
        "forbid(unsafe_code)",
        "Truth/appearance seam",
        "alacritty_terminal",
    ] {
        assert!(
            decisions.contains(subject),
            "decisions.md must record an ADR about `{subject}`"
        );
    }
}

/// T-002 EARS: the crate root's first line forbids unsafe (guarantee #1).
#[test]
fn test_crate_root_forbids_unsafe() {
    let main = repo_file("src/main.rs");
    let first = main.lines().next().unwrap_or_default().trim();
    assert_eq!(
        first, "#![forbid(unsafe_code)]",
        "src/main.rs line 1 must be #![forbid(unsafe_code)]"
    );
}
