# Agent Tasks (Persistent Backlog)

- [ ] T-1102 (sprint 11): Add ResolvedShell + into_command() in pty.rs (pure, get_argv-based) — touches: src/core/pty.rs
- [ ] T-1103 (sprint 11): Add pure resolve_shell(config, name) -> Option<ResolvedShell> — touches: src/core/shell.rs (new), src/core/mod.rs
- [ ] T-1104 (sprint 11): Add os::detect_shells() + pure detect_in(paths) PATH probe with guaranteed fallback — touches: src/os/mod.rs, src/os/windows.rs, src/os (unix arm)
- [ ] T-1105 (sprint 11): Refactor open_pty + session::spawn to accept Option<&ResolvedShell> (None = new_default_prog) — touches: src/core/pty.rs, src/core/session.rs
- [ ] T-1106 (sprint 11): Wire main.rs startup + all app.rs new-tab sites to resolved default stored on BanquoApp — touches: src/main.rs, src/app.rs
- [ ] T-1107 (sprint 11): Command-palette `shell <name>` verb (new tab w/ profile; unknown = safe no-op) — touches: src/app.rs
- [ ] T-1108 (sprint 11): Add #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] — touches: src/main.rs
- [ ] T-1109 (sprint 11): install.ps1 — release build + copy + shortcut (abort on build failure) — touches: install.ps1 (new)
- [ ] T-1110 (sprint 11): README "Run it" rewrite + ADR (record handoff + elevation deferrals) — touches: README.md, decisions.md
