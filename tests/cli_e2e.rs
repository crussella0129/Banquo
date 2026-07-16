//! End-to-end tests for Banquo's console CLI.
//!
//! Each test drives the real binary (`CARGO_BIN_EXE_banquo`) with
//! `BANQUO_CONFIG` pointed into a private temp directory, so tests are
//! hermetic, parallel-safe, and never touch the developer's real config.

use std::path::PathBuf;
use std::process::{Command, Output};

/// A private temp workspace for one test.
struct TestHome {
    dir: PathBuf,
}

impl TestHome {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "banquo_e2e_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn config_path(&self) -> PathBuf {
        self.dir.join("banquo.toml")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_banquo"))
            .args(args)
            .env("BANQUO_CONFIG", self.config_path())
            .output()
            .expect("banquo binary runs")
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn all_output(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn test_e2e_check_valid_config_exit0() {
    let home = TestHome::new("check_valid");
    std::fs::write(home.config_path(), "theme = \"zircon\"\n").unwrap();
    let out = home.run(&["check"]);
    assert!(out.status.success(), "output: {}", all_output(&out));
    assert!(
        stdout(&out).contains("config ok"),
        "output: {}",
        all_output(&out)
    );
}

#[test]
fn test_e2e_check_invalid_toml_nonzero() {
    let home = TestHome::new("check_invalid");
    std::fs::write(home.config_path(), "= bad").unwrap();
    let out = home.run(&["check"]);
    assert!(!out.status.success());
    assert!(
        all_output(&out).contains("TOML parse error"),
        "output: {}",
        all_output(&out)
    );
}

#[test]
fn test_e2e_check_missing_config_ok() {
    let home = TestHome::new("check_missing");
    let out = home.run(&["check"]);
    assert!(out.status.success());
    assert!(
        stdout(&out).contains("defaults in effect"),
        "output: {}",
        all_output(&out)
    );
}

#[test]
fn test_e2e_preset_list_contains_builtins() {
    let home = TestHome::new("preset_list");
    let out = home.run(&["preset", "list"]);
    assert!(out.status.success());
    let text = stdout(&out);
    for name in [
        "zircon",
        "blanco",
        "concrete",
        "concrete-dark",
        "primordial",
        "volcanic-glass",
    ] {
        assert!(text.contains(name), "missing {name} in: {text}");
    }
}

#[test]
fn test_e2e_preset_list_marks_user_presets() {
    let home = TestHome::new("preset_list_user");
    let presets_dir = home.dir.join("presets");
    std::fs::create_dir_all(&presets_dir).unwrap();
    std::fs::write(presets_dir.join("mytheme.toml"), "theme = \"mytheme\"\n").unwrap();
    let out = home.run(&["preset", "list"]);
    assert!(out.status.success());
    assert!(
        stdout(&out).contains("mytheme (user)"),
        "output: {}",
        stdout(&out)
    );
}

#[test]
fn test_e2e_config_init_creates_file() {
    let home = TestHome::new("config_init");
    let out = home.run(&["config", "init", "--preset", "zircon"]);
    assert!(out.status.success(), "output: {}", all_output(&out));
    let content = std::fs::read_to_string(home.config_path()).unwrap();
    assert!(content.contains("theme = \"zircon\""));

    // Second init without --force must refuse.
    let out2 = home.run(&["config", "init"]);
    assert!(!out2.status.success());
    assert!(
        all_output(&out2).contains("--force"),
        "output: {}",
        all_output(&out2)
    );

    // And with --force it overwrites.
    let out3 = home.run(&["config", "init", "--preset", "blanco", "--force"]);
    assert!(out3.status.success());
    let content3 = std::fs::read_to_string(home.config_path()).unwrap();
    assert!(content3.contains("theme = \"blanco\""));
}

#[test]
fn test_e2e_config_path_honors_env() {
    let home = TestHome::new("config_path");
    let out = home.run(&["config", "path"]);
    assert!(out.status.success());
    assert_eq!(
        stdout(&out).trim(),
        home.config_path().display().to_string()
    );
}

#[test]
fn test_e2e_config_show_roundtrips() {
    let home = TestHome::new("config_show");
    std::fs::write(
        home.config_path(),
        "theme = \"concrete\"\n[fonts]\nsize = 18.0\n",
    )
    .unwrap();
    let out = home.run(&["config", "show"]);
    assert!(out.status.success());
    let text = stdout(&out);
    // The printed TOML must itself be valid TOML carrying the same values.
    let value: toml::Value = toml::from_str(&text).expect("config show output is valid TOML");
    assert_eq!(
        value.get("theme").and_then(|v| v.as_str()),
        Some("concrete")
    );
    // Tables must survive the round-trip too (dotfiles-export contract).
    assert_eq!(
        value
            .get("fonts")
            .and_then(|f| f.get("size"))
            .and_then(|v| v.as_float()),
        Some(18.0)
    );
}

#[test]
fn test_e2e_check_error_diagnostic_nonzero() {
    // Valid TOML but an Error diagnostic (unresolved shell.default) — a
    // different branch than the parse-error path.
    let home = TestHome::new("check_error_diag");
    std::fs::write(home.config_path(), "[shell]\ndefault = \"ghost\"\n").unwrap();
    let out = home.run(&["check"]);
    assert!(!out.status.success(), "output: {}", all_output(&out));
    assert!(
        stdout(&out).contains("error:"),
        "output: {}",
        all_output(&out)
    );
}

#[test]
fn test_e2e_check_warning_only_exit0() {
    // Warnings alone (unknown theme) must not fail the check.
    let home = TestHome::new("check_warning_only");
    std::fs::write(home.config_path(), "theme = \"mytheme\"\n").unwrap();
    let out = home.run(&["check"]);
    assert!(out.status.success(), "output: {}", all_output(&out));
    assert!(
        stdout(&out).contains("warning:"),
        "output: {}",
        all_output(&out)
    );
}

#[test]
fn test_e2e_preset_apply_preserves_shell() {
    let home = TestHome::new("preset_apply");
    std::fs::write(
        home.config_path(),
        r#"
theme = "zircon"

[fonts]
size = 22.0

[shell]
default = "pwsh"

[[shell.profiles]]
name = "pwsh"
command = "pwsh.exe"
"#,
    )
    .unwrap();

    let out = home.run(&["preset", "apply", "blanco"]);
    assert!(out.status.success(), "output: {}", all_output(&out));

    let content = std::fs::read_to_string(home.config_path()).unwrap();
    let value: toml::Value = toml::from_str(&content).unwrap();
    // Preset took effect...
    assert_eq!(value.get("theme").and_then(|v| v.as_str()), Some("blanco"));
    // ...and the user's shell + fonts survived the merge.
    assert_eq!(
        value
            .get("shell")
            .and_then(|s| s.get("default"))
            .and_then(|v| v.as_str()),
        Some("pwsh")
    );
    assert_eq!(
        value
            .get("fonts")
            .and_then(|f| f.get("size"))
            .and_then(|v| v.as_float()),
        Some(22.0)
    );
}

#[test]
fn test_e2e_preset_apply_unknown_nonzero() {
    let home = TestHome::new("preset_apply_unknown");
    let out = home.run(&["preset", "apply", "nope"]);
    assert!(!out.status.success());
    assert!(
        all_output(&out).contains("unknown preset"),
        "output: {}",
        all_output(&out)
    );
}

#[test]
fn test_e2e_compose_alias() {
    let home = TestHome::new("compose_alias");
    std::fs::write(home.config_path(), "theme = \"zircon\"\n").unwrap();
    let out = home.run(&["compose", "--check"]);
    assert!(out.status.success(), "output: {}", all_output(&out));
    assert!(stdout(&out).contains("config ok"));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("deprecated"),
        "compose should print a deprecation note"
    );

    // "Behaves as check" includes the failure contract.
    std::fs::write(home.config_path(), "= bad").unwrap();
    let out2 = home.run(&["compose", "--check"]);
    assert!(
        !out2.status.success(),
        "compose must propagate check's non-zero exit: {}",
        all_output(&out2)
    );
}
