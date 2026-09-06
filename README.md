# claude-nav

How Claude Code agents read, and how you get to one.

```console
$ claude-nav list
{
  "taken_at": 1788721509,
  "waiting": 1,
  "agents": [
    { "title": "infra", "status": "waiting", "state": "waiting", "glyph": "🙋",
      "spinner": false, "age_s": 47, "age": "47m",
      "target": { "session": "infra", "pane": "0" } }
  ]
}

$ claude-nav jump infra 0
```

Two commands. `list` prints every live agent in the order a person reads them,
and `jump` moves the focus to one.

## Why it exists

[`claude-ps`](https://github.com/lorenzolfm/claude-ps) reports what is running.
It says an agent's status is `waiting`, and it passes that word through
unchanged, and it tells consumers not to compare it against a fixed set.

Somebody still has to decide what `waiting` *means*: that it sorts above `busy`,
that it is the one status worth a number in a bar, that a `derived` name loses to
the zellij session that holds it, that an unknown word is shown and never
counted. Those are decisions, and there are three surfaces that need them —
[`luneta`](https://github.com/lorenzolfm/luneta)'s agents tab, the
[`claude-tray`](https://github.com/lorenzolfm/claude-tray) applet, and the
vicinae extension.

Three copies of that table would be three answers. The failure is not abstract:
an earlier version of the applet added two states of its own, `your turn` and
`dormant`, with three thresholds that the picker knew nothing about, and the same
agent then read as `your turn` on one surface and `idle` on the other. A
vocabulary that differs between two surfaces is worse than no vocabulary.

The jump has the same shape. It is 600 lines of socket pairing, process ancestry
and compositor calls, and each line of it was written against a failure that was
silent. A second implementation would rediscover them one at a time.

## Two doors, one implementation

| | reaches it by | gets |
|---|---|---|
| `claude-tray` | linking the crate | `Entry`, `Badge`, `State`, `jump::focus` |
| the vicinae extension | running the binary | `claude-nav list`, `claude-nav jump` |

A Rust consumer keeps compile-time types and no subprocess in its poll loop. A
consumer with no Rust in it reads JSON. Neither one decides anything.

## What a row is called

`claude-ps` reports the `name` of the agent and who chose it, and the second
value is the important one. A row takes a name in three steps:

1. the name that a person chose (a `name_source` of `user` or `peer`), which is
   the only string in the row that gives the purpose of the agent;
2. or the zellij session that holds the agent, which is what you navigate by;
3. or Claude Code's own label, for an agent outside zellij that would otherwise
   have no name.

A `derived` name is the cwd basename plus a two-character suffix. A row with that
name would carry the name of a directory that holds it by chance:
`…/infra.git/master` would show `master-3c` for a session that you reach as
`infra`.

An unknown `name_source` is suppressed, which is the opposite of the treatment of
an unknown status. The producer causes that difference, and it is correct on both
sides. Each status value is a real state, so a hidden value hides a live agent.
But the sources that carry a chosen name are a short closed list (`user`,
`peer`), and the sources that carry a generated name are a long open one: Claude
Code already writes `derived`, `collision`, `auto` and `hook`. A new source is
more probably a generated name. An absent source is trusted, because it is the
state from before the key existed and not a value that this build failed to
recognise.

When two rows take the same name, each of those rows takes a `:pane` suffix. A
rule that leaves the first row without one is not a rule that a person can see.

## What it decides

| raw `status` | | `state` | `glyph` | sorts | counted |
|---|---|---|---|---|---|
| `waiting` | | `waiting` | 🙋 | 1st | ✅ |
| `idle` | | `idle` | ☕ | 2nd | — |
| `busy` | | `busy` | *null* | 3rd | — |
| `shell` | | `other` | 🐚 | last | — |
| anything else | | `other` | 🛸 | last | — |

`status` is what Claude Code said, verbatim, and its vocabulary is open. `state`
is this program's closed reading of it, and it is what a consumer sorts and
counts by.

Three rules are important, and a test verifies each one:

- **An unknown status sorts last and is never counted.** The vocabulary changes
  with the releases of Claude Code, which added `shell` between two of them, so a
  new word is only a question of time. Last place fails quietly, but a count
  would make a badge out of a spelling error. The row is still shown, with the
  status itself as its word.
- **Case never decides what a status is.** The comparison is
  `eq_ignore_ascii_case` on every surface. A status that one surface recognises
  and another does not is a second mapping.
- **Nothing is hidden.** Uncounted is not absent: every live agent takes a row.
  The only rows with nothing behind them are agents outside zellij, whose
  `target` is null.

### `waiting` is the only status that counts

`waiting` means the agent asked you something and stopped, so `waiting > 0` means
an agent needs you.

`idle` is not counted at any age. A finished turn is enough for a row above the
working rows, which is why it sorts second, but it is not enough for a number,
because nothing ends that state. An earlier design counted `idle` for its first
hour and suppressed it for the first thirty seconds of a session's life; both
values were estimates of how long a person stays interested. Counting `waiting`
alone needs neither.

### `busy` has no glyph

`glyph` is null for exactly the rows where `spinner` is true, and the caller
draws its own frames. Which status turns is decided here; how many dots turn is
not.

This is not a detail. The applet spends `⣾⣽⣻⢿⡿⣟⣯⣷`, seven dots to a frame,
where luneta and the launcher spend `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`, which is three. A GTK menu
draws its rows in the theme's foreground colour, and three dots become grey
specks there that may or may not turn. Colour is the smaller change and is not
available: a tray menu goes through `libdbusmenu-gtk3`, which calls
`g_markup_escape_text` on each label, so markup for one glyph shows as angle
brackets.

## Ordering

Attention first, and most recent first within one status. The agent that changed
a moment ago is the agent you last worked with.

`claude-ps` sorts by pid, so that two runs one second apart give a clean
difference, and its README says that order is for comparison and not for reading.
Equal ages keep it, so two agents that changed in the same second do not exchange
places between polls.

The rows are one flat list. There are no dividers between groups, because the
glyph and the word already show the group of a row.

A surface that re-sorts this list has replaced it. That includes a fuzzy search
that ranks what it keeps: narrowing a list is not the same as rearranging it, and
two surfaces that disagree about the first row are worse than either rule alone.

## Ages

`age_s` is whole seconds in the current status. On a `busy` row that is the
duration of the turn, and it is the only indication that a session is stuck.

`age` is the same number written the way every surface writes it: `<1m`, `47m`,
`3h`, `2d`. It is on the wire so that no consumer has to hold a second copy of
the format.

`taken_at` is when `claude-ps` ran. A consumer that polls more slowly than it
redraws must add the elapsed time to each `age_s`, or an agent that has waited
three minutes shows the same number for as long as you look at it — in the one
column that reports whether an agent is stuck. The addition is safe because it is
the same number in every row, and an equal offset cannot change a comparison.

## Jumping to a pane

`claude-nav jump <session> <pane>` is silent on success and exits non-zero with a
line on stderr otherwise.

It refuses an address that no live agent holds, and that check belongs to the
command line rather than to the library. A tray menu satisfies the contract by
construction: a row carries the address of the snapshot it was drawn from, and
the menu is rebuilt as it opens. A shell satisfies nothing, and a launcher's list
can be a second old by the time `Enter` reaches it. Without the check, a session
that exited in that second falls through to `attach`, which *creates* one under
that name and returns success — the jump reports that it landed, having invented
the place. It is the whitespace failure again, reached by a different door.

There is one terminal and many sessions, so the rule is to change the session in
the terminal that you have, and to open a terminal only if there is none. There
is no rule to select between windows, because there is nothing to select.

`pane` is `$ZELLIJ_PANE_ID`, which is what `zellij action focus-pane-id` takes,
addressed at a session by name from outside it. `claude-ps` sends it with the
session in one object, or it sends null, so a row has a full address or none.

Four things here were each a silent failure:

- **The live session comes from the socket, not from `argv`.** The `argv` of a
  zellij client names the session it attached to first, not the one it shows now.
  A client connects to exactly one `zellij --server <path>/<session>`, so pairing
  the two ends of that socket gives the session. `/proc/net/unix` does not hold
  peer inodes and only `ss` does, over sock_diag netlink, so the jump runs `ss`.
- **A session name with whitespace is refused.** The pairing splits `ss -x -p`
  rows on whitespace, which holds for every socket path zellij builds and not for
  every session name a person types. A session named `my work` would be joined to
  no client, and the jump would fall through to `attach` and open a *second*
  terminal for a session already on the screen — and the click would look like it
  worked. Such an address is refused instead, by `list` and by `jump` alike.
- **The switch is verified, not assumed.** zellij sends `switch-session` to the
  last client that pressed a key in that session, and a client that arrived by a
  change of session has pressed none. The jump therefore sends a `Ctrl e` pair,
  which is a binding zellij consumes and which never reaches the pane, and then
  reads the client's live session back. `zellij` exits 0 when the session does not
  exist and prints the session list instead; only a wrong pane id exits non-zero.
  A focus that succeeds is silent on both streams, so this code tests for silence.
- **`hyprctl` needs a signature this process cannot inherit.** The jump asks
  Hyprland which pids own windows and then raises one, and `hyprctl` finds the
  compositor through `HYPRLAND_INSTANCE_SIGNATURE`. A process started by the
  systemd user manager has its environment fixed before the compositor exists and
  never receives it. The failure is silent too: `hyprctl` prints
  `HYPRLAND_INSTANCE_SIGNATURE not set!` on stdout and exits 0, so no code here
  reads an exit code from it. Each call examines the output instead.

  The signature is therefore found here, in `$XDG_RUNTIME_DIR/hypr`, which holds
  one directory per instance with the signature as its name. A directory is not
  an instance but a socket is: Hyprland leaves the directory and its log behind
  when it exits, so a machine that restarted the compositor has several and only
  one accepts a connection. A test for `.socket.sock` removes the stale ones and
  the newest of the remainder is live. An environment that has the variable
  always wins, because that is the compositor the person is looking at.

## Install

```sh
nix profile install github:lorenzolfm/claude-nav
```

Nothing is pinned into the package, and the list is longer than it looks:
`claude-ps`, `zellij`, `ss`, `hyprctl` and a terminal all come from `PATH`.

`claude-ps` has its own release cadence and must stay upgradeable without a
rebuild here. The other three are worse than merely inconvenient to pin: the jump
speaks to a *running* zellij server and a *running* compositor, and a second
build of either from a different store path could answer incorrectly. This
program has to reach the ones the person is looking at.

A consumer that runs `claude-nav` from a systemd user unit must therefore set the
unit's `PATH`. NixOS gives a user unit coreutils, findutils, grep, sed and
systemd, and none of the five above.

## Licence

MIT.
