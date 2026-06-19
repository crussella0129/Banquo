//! PTY spawning — the OS boundary of the truth half.
//!
//! `open_pty` spawns the platform's default shell on a pseudo-terminal and
//! returns the reader, writer, and master handles. On Windows this uses ConPTY
//! (Windows 10 1809+); on Unix, `openpty`/`forkpty`. The abstraction is
//! `portable-pty`, the same one WezTerm ships (design §VIII).

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};

/// A fully-resolved shell spec, ready to spawn on a PTY.
///
/// This is the GUI-free, pure description of *which program to run* — produced
/// by the resolver from a configured `ShellProfile` (or synthesized for the OS
/// default). Keeping it a plain struct lets the resolution logic be unit-tested
/// without ever spawning a process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedShell {
    /// The program to launch (becomes `argv[0]`).
    pub prog: String,
    /// Arguments after the program.
    pub args: Vec<String>,
    /// Working directory, if overridden.
    pub cwd: Option<String>,
    /// Extra environment variables to set (key, value) pairs.
    pub env: Vec<(String, String)>,
}

impl ResolvedShell {
    /// Build a `portable_pty::CommandBuilder` from this spec.
    ///
    /// `portable-pty` models the program as `argv[0]`, so we construct with
    /// `CommandBuilder::new(prog)` and append `args`. `cwd`/`env` are only set
    /// when present, so an empty spec inherits Banquo's environment unchanged.
    /// Takes `&self` (not `self`) because one `ResolvedShell` is reused to spawn
    /// many tabs, so the `to_command` (not `into_command`) naming is correct.
    pub fn to_command(&self) -> CommandBuilder {
        let mut cmd = CommandBuilder::new(&self.prog);
        cmd.args(&self.args);
        if let Some(cwd) = &self.cwd {
            cmd.cwd(cwd);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        cmd
    }
}

/// The handles returned by [`open_pty`].
pub struct PtyHandle {
    /// Read end — the reader thread reads PTY output from here.
    pub reader: Box<dyn Read + Send>,
    /// Write end — the Face writes keystrokes here.
    pub writer: Box<dyn Write + Send>,
    /// The master handle — used for resize.
    pub master: Box<dyn MasterPty + Send>,
    /// The spawned child process.
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

/// Spawn a shell on a PTY of the given dimensions.
///
/// When `shell` is `Some`, that resolved profile is launched; when `None`,
/// Banquo falls back to the OS default program (`new_default_prog`) — the
/// pre-sprint-11 behavior. Returns a [`PtyHandle`] with reader, writer, master
/// (for resize), and the child process. Returns `Err` if the PTY cannot be
/// opened or the shell cannot be spawned (no panic — the caller decides how to
/// report it).
pub fn open_pty(cols: u16, rows: u16, shell: Option<&ResolvedShell>) -> anyhow::Result<PtyHandle> {
    let pty_system = native_pty_system();

    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = match shell {
        Some(s) => s.to_command(),
        None => CommandBuilder::new_default_prog(),
    };

    // Explicitly announce UTF-8 / TrueColor support to shells and tools
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("LANG", "en_US.UTF-8");
    cmd.env("LC_ALL", "en_US.UTF-8");

    // Some tools on Windows detect Windows Terminal capabilities via this variable:
    cmd.env("WT_SESSION", "1");

    let child = pair.slave.spawn_command(cmd)?;

    // Drop the slave — we only need the master side.
    drop(pair.slave);

    let reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;

    Ok(PtyHandle {
        reader,
        writer,
        master: pair.master,
        child,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn test_into_command_sets_prog_and_args() {
        let shell = ResolvedShell {
            prog: "pwsh.exe".to_string(),
            args: vec!["-NoLogo".to_string()],
            cwd: None,
            env: vec![],
        };
        let cmd = shell.to_command();
        // portable-pty has no get_prog(); the program is argv[0].
        let argv = cmd.get_argv();
        assert_eq!(argv[0], OsString::from("pwsh.exe"));
        assert_eq!(argv[1], OsString::from("-NoLogo"));
    }

    #[test]
    fn test_into_command_omits_empty_cwd_env() {
        let shell = ResolvedShell {
            prog: "cmd.exe".to_string(),
            args: vec![],
            cwd: None,
            env: vec![],
        };
        let cmd = shell.to_command();
        assert!(cmd.get_cwd().is_none());
        // env-omission: with no profile env, our marker var is absent.
        assert!(cmd.get_env("BANQUO_TEST_VAR").is_none());
    }

    #[test]
    fn test_into_command_populates_cwd_env() {
        // The populated path must actually reach the CommandBuilder (guards the
        // `cmd.cwd(..)` branch and the `for (k,v) in &self.env` loop).
        let shell = ResolvedShell {
            prog: "wsl.exe".to_string(),
            args: vec![],
            cwd: Some("C:/work".to_string()),
            env: vec![("BANQUO_TEST_VAR".to_string(), "1".to_string())],
        };
        let cmd = shell.to_command();
        assert_eq!(cmd.get_cwd(), Some(&OsString::from("C:/work")));
        assert_eq!(
            cmd.get_env("BANQUO_TEST_VAR"),
            Some(OsString::from("1").as_os_str())
        );
    }
}
