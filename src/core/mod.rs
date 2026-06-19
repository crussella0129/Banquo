//! The `core` module — Banquo's truth half (design §I, §IV).
//!
//! Everything here is GUI-unaware: PTY bytes → parser → grid → cursor →
//! scrollback, published as immutable [`snapshot::Snapshot`] frames. The Face
//! reads these snapshots; it never writes them. No `alacritty_*` type crosses
//! this module boundary (ADR-003/004) — the adapter lives inside [`term`].

pub mod pty;
pub mod session;
pub mod shell;
pub mod snapshot;
pub mod term;
