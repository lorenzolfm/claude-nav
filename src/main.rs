//! The door for a consumer with no Rust in it.
//!
//! `list` prints the rows that `state` decided, and `jump` moves the focus to
//! one of them. A consumer that reads this output renders it and decides
//! nothing, which is the point: the launcher extension and the tray applet show
//! the same agent in the same place with the same word.

use claude_nav::agents::{self, Row, Zellij};
use claude_nav::jump;
use claude_nav::state::{self, Snapshot, Target};
use serde::Serialize;
use std::process::ExitCode;

const USAGE: &str = "\
claude-nav — how Claude Code agents read, and how you get to one.

  claude-nav list
      Print every live agent on stdout as a JSON object, in the order a person
      reads them: the ones that wait for you first, most recent first within a
      status. Exits non-zero if claude-ps cannot be run.

  claude-nav jump <session> <pane>
      Move the focus to that zellij pane, raising or retargeting the terminal
      that shows it and opening one only if there is none. Silent on success.
      Exits non-zero if no live agent holds that pane.

The `status` of a row is what Claude Code reports, verbatim, and its vocabulary
is open. `state` is this program's closed reading of it — waiting, idle, busy or
other — and it is what you sort and count by. A word this build does not know is
`other`: shown, never counted, sorted last.
";

/// One agent, as a surface should show it.
#[derive(Debug, Serialize)]
struct Agent {
    /// What to call the row: the name a person chose, else the zellij session,
    /// else Claude Code's own label.
    title: String,
    /// What Claude Code reports, unchanged. Open vocabulary: do not match on it.
    status: String,
    /// This program's reading of `status`. Closed: `waiting`, `idle`, `busy`,
    /// `other`. Sort and count by this.
    state: &'static str,
    /// The glyph for a status that does not turn, or `null` for `busy`, whose
    /// spinner belongs to whichever surface draws it.
    glyph: Option<&'static str>,
    /// Whether this row turns. True for exactly the rows whose `glyph` is null.
    spinner: bool,
    /// Whole seconds in the current status, as measured at `taken_at`.
    age_s: u64,
    /// `age_s` written the way every surface writes it: `<1m`, `47m`, `3h`, `2d`.
    age: String,
    /// Where `jump` would go, or `null` for a row with no address — an agent
    /// outside zellij, or a session whose name holds whitespace. A row with a
    /// null target has nowhere to go and should be inert.
    target: Option<Address>,
}

#[derive(Debug, Serialize)]
struct Address {
    session: String,
    pane: String,
}

#[derive(Debug, Serialize)]
struct Listing {
    /// Epoch seconds when claude-ps was run. A consumer that polls more slowly
    /// than it renders can add the elapsed time to each `age_s`.
    taken_at: u64,
    /// How many agents wait for you. Counted here so that a consumer cannot
    /// disagree with its own list about the number.
    waiting: usize,
    agents: Vec<Agent>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();

    match argv.as_slice() {
        ["list"] => list(),
        ["jump", session, pane] => leap(session, pane),
        ["-h" | "--help" | "help"] | [] => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn list() -> ExitCode {
    let rows = match agents::poll() {
        Ok(rows) => rows,
        Err(e) => return fail(&e.to_string()),
    };

    let snap = state::snapshot(&rows, now());
    let json = match serde_json::to_string_pretty(&listing(&snap)) {
        Ok(json) => json,
        Err(e) => return fail(&format!("could not write the listing: {e}")),
    };

    println!("{json}");
    ExitCode::SUCCESS
}

fn listing(snap: &Snapshot) -> Listing {
    Listing {
        taken_at: snap.taken_at,
        waiting: snap.badge().waiting().map_or(0, |n| n.get()),
        agents: snap
            .entries
            .iter()
            .map(|entry| Agent {
                title: entry.title.clone(),
                status: entry.status.as_str().to_string(),
                state: word(entry.status.state()),
                glyph: entry.status.fixed_glyph(),
                spinner: entry.status.is_spinning(),
                age_s: entry.age_s,
                age: state::humanise(entry.age_s),
                target: entry.target.as_ref().map(|t| Address {
                    session: t.session.clone(),
                    pane: t.pane.clone(),
                }),
            })
            .collect(),
    }
}

/// The closed half of the vocabulary, written out. A consumer matches on these
/// four words and on nothing in `status`.
fn word(state: state::State) -> &'static str {
    match state {
        state::State::Waiting => "waiting",
        state::State::Idle => "idle",
        state::State::Busy => "busy",
        state::State::Other => "other",
    }
}

fn leap(session: &str, pane: &str) -> ExitCode {
    let target = Target {
        session: session.to_string(),
        pane: pane.to_string(),
    };

    // The same refusal the list makes: an address that `list` would have shown
    // as null is not one this program will act on. Without it, a session named
    // with whitespace falls through to `attach` and opens a second terminal for
    // a session already on the screen, and the jump looks like it worked.
    let usable = |s: &str| !s.is_empty() && !s.contains(char::is_whitespace);
    if !usable(session) || !usable(pane) {
        return fail(&format!("{session}:{pane} is not an address"));
    }

    // The library's contract is that the address names a live agent, and the
    // tray applet satisfies it by construction: a menu row carries the address
    // of the snapshot it was drawn from, and the menu is rebuilt as it opens.
    //
    // A command line satisfies nothing, and a launcher's list can be a second
    // old when `Enter` reaches it. Without this check an agent that has exited
    // since the list was drawn falls through to `attach`, which *creates* a
    // session under that name and returns success — the same failure that the
    // whitespace rule above exists to prevent, reached by a different door. The
    // jump would report that it landed, and it would have invented the place.
    match agents::poll() {
        Ok(rows) if !holds(&rows, session, pane) => {
            return fail(&format!("no agent at {session}:{pane}"));
        }
        Err(e) => return fail(&e.to_string()),
        Ok(_) => {}
    }

    match jump::focus(&target) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(&e),
    }
}

/// Whether a live agent holds that pane.
///
/// The comparison is against the address a row would have been *shown* with, so
/// an address `list` refused is one `jump` refuses too. Two doors, one answer.
fn holds(rows: &[Row], session: &str, pane: &str) -> bool {
    rows.iter()
        .filter_map(|row| row.zellij.as_ref())
        .filter_map(Zellij::address)
        .any(|z| z.session == session && z.pane == pane)
}

fn fail(why: &str) -> ExitCode {
    eprintln!("claude-nav: {why}");
    ExitCode::FAILURE
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(session: &str, pane: &str) -> Row {
        Row {
            raw_status: "idle".into(),
            transition_age_s: 5,
            zellij: Some(Zellij {
                session: session.into(),
                pane: pane.into(),
            }),
            name: "n".into(),
            name_source: None,
        }
    }

    #[test]
    fn a_live_pane_is_held() {
        let rows = [row("infra", "0"), row("dotfiles", "3")];
        assert!(holds(&rows, "infra", "0"));
        assert!(holds(&rows, "dotfiles", "3"));
    }

    #[test]
    fn a_session_that_is_gone_holds_nothing() {
        assert!(!holds(&[row("infra", "0")], "nosuchsession", "0"));
        assert!(!holds(&[], "infra", "0"));
    }

    #[test]
    fn the_pane_has_to_match_too() {
        assert!(!holds(&[row("infra", "0")], "infra", "3"));
    }

    #[test]
    fn an_agent_outside_zellij_holds_nothing() {
        let mut outside = row("infra", "0");
        outside.zellij = None;
        assert!(!holds(&[outside], "infra", "0"));
    }

    #[test]
    fn an_address_the_listing_refused_is_refused_here() {
        // `list` shows a null target for these, so `jump` must not act on them
        // either: one of the two would otherwise invent a session by attaching.
        for (session, pane) in [("my work", "0"), ("", "0"), ("infra", "")] {
            assert!(
                !holds(&[row(session, pane)], session, pane),
                "{session}:{pane}"
            );
        }
    }
}
