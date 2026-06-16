//! The session — the reader thread and the snapshot publish pipeline.
//!
//! `Session::spawn` ties together the PTY (reader/writer/master), the term
//! wrapper, and the `ArcSwap` publish mechanism (design §IV). A dedicated OS
//! thread reads PTY output, advances the terminal state machine, builds an
//! immutable [`Snapshot`], and publishes it via `ArcSwap::store`. The Face
//! loads the latest snapshot each frame — never locking the live grid
//! (guarantee #2: the core never blocks the frame).
//!
//! Read bursts are coalesced: the reader drains as many bytes as available in
//! a single pass before building and publishing one snapshot. This means a
//! `cat` of a 2GB file falls *behind* but never *freezes* the window.

use std::io::{Read, Write};
use std::sync::Arc;
use std::thread;

use arc_swap::ArcSwap;
use portable_pty::{MasterPty, PtySize};

use super::snapshot::Snapshot;
use super::term::BanquoTerm;

/// The handle the Face holds after the session is spawned.
///
/// This is the **only** cross-thread surface: the Face reads the snapshot
/// (lock-free via `ArcSwap::load`), writes keystrokes to the PTY writer, and
/// sends resize commands. It never touches the live `Term`.
pub struct SessionHandle {
    /// The latest published snapshot. Load with `snapshot.load()`.
    pub snapshot: Arc<ArcSwap<Snapshot>>,
    /// Write end of the PTY — the Face writes encoded keystrokes here.
    pub writer: Box<dyn Write + Send>,
    /// The master PTY handle — used for resize.
    master: Box<dyn MasterPty + Send>,
    /// The term wrapper — used for resize (must resize both PTY and term).
    term: Arc<std::sync::Mutex<BanquoTerm>>,
    /// The spawned child process.
    #[allow(dead_code)] // Kept for process lifecycle management.
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl SessionHandle {
    /// Resize the terminal: drives the PTY master *and* the term together.
    /// The PTY master sends `SIGWINCH` (or ConPTY equivalent) to the shell;
    /// the term reflows its grid.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        // Resize the PTY first (so the shell gets SIGWINCH)...
        let _ = self.master.resize(PtySize {
            rows: rows as u16,
            cols: cols as u16,
            pixel_width: 0,
            pixel_height: 0,
        });
        // ...then resize the term (so the grid reflows to match).
        if let Ok(mut term) = self.term.lock() {
            term.resize(cols, rows);
        }
    }
}

/// Spawn a terminal session: PTY + reader thread + snapshot publisher.
///
/// Returns a [`SessionHandle`] on success, or an error if the PTY can't be
/// opened. The reader thread runs until the PTY reader returns EOF (shell
/// exits) or the process is killed.
pub fn spawn(cols: usize, rows: usize) -> anyhow::Result<SessionHandle> {
    let pty = super::pty::open_pty(cols as u16, rows as u16)?;

    let term = BanquoTerm::new(cols, rows);
    let term = Arc::new(std::sync::Mutex::new(term));

    let initial = Snapshot::empty(cols, rows);
    let snapshot = Arc::new(ArcSwap::from_pointee(initial));

    // Clone for the reader thread.
    let reader_term = Arc::clone(&term);
    let reader_snapshot = Arc::clone(&snapshot);
    let mut reader = pty.reader;

    thread::Builder::new()
        .name("banquo-pty-reader".into())
        .spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF — shell exited
                    Ok(n) => {
                        if let Ok(mut term) = reader_term.lock() {
                            term.advance(&buf[..n]);
                            let snap = term.build_snapshot();
                            reader_snapshot.store(Arc::new(snap));
                        }
                    }
                    Err(_) => break, // Read error — PTY closed
                }
            }
        })?;

    Ok(SessionHandle {
        snapshot,
        writer: pty.writer,
        master: pty.master,
        term,
        child: pty.child,
    })
}
