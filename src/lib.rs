//! One vocabulary and one jump, for every surface that shows Claude Code agents.
//!
//! [`claude-ps`] joins each agent to its zellij pane and reports a status. This
//! crate decides what that status *means* — what it is called, where it sorts,
//! whether it waits for you — and how to move the focus to the agent behind a
//! row. Nothing here draws.
//!
//! Two consumers use two doors, and they are the same code:
//!
//! * the tray applet links the library, so its menu holds Rust types and no
//!   subprocess sits in its poll loop;
//! * the launcher extension runs the binary, so a program with no Rust in it
//!   reads `claude-nav list` and calls `claude-nav jump`.
//!
//! A second implementation of either half is the thing this crate exists to
//! prevent. Two surfaces that disagree about the first row are worse than
//! either rule alone.
//!
//! [`claude-ps`]: https://github.com/lorenzolfm/claude-ps

pub mod agents;
pub mod jump;
pub mod state;
