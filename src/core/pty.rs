//! PTY spawning — the OS boundary of the truth half.
//!
//! `open_pty` spawns the platform's default shell on a pseudo-terminal and
//! returns the reader, writer, and master handles. On Windows this uses ConPTY
//! (Windows 10 1809+); on Unix, `openpty`/`forkpty`. The abstraction is
//! `portable-pty`, the same one WezTerm ships (design §VIII).

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};

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

/// Spawn the default shell on a PTY of the given dimensions.
///
/// Returns a [`PtyHandle`] with reader, writer, master (for resize), and the
/// child process. Returns `Err` if the PTY cannot be opened or the shell
/// cannot be spawned (no panic — the caller decides how to report it).
pub fn open_pty(cols: u16, rows: u16) -> anyhow::Result<PtyHandle> {
    let pty_system = native_pty_system();

    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new_default_prog();

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
