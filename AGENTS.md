# Spotsie agent guide

Spotsie is a native Spotify client that started from the code of
[Spotifast](https://github.com/crmne/spotifast) (MIT). It keeps that proven
backend (librespot playback, Connect, sign-in, the Web API, settings and
storage) and has its own interface, built to look and feel like Spotify's
desktop app, without features that do not serve that.

Follow `CONTRIBUTING.md`; it is the canonical product and contribution policy.
These instructions add implementation constraints for coding agents.

## Where the work goes

- **Interface: change freely.** `src/ui/`, `src/theme.rs`, layout, spacing,
  navigation, typography and visual hierarchy are where Spotsie's work goes.
  Redesigns are the point; describe every user-visible change in the commit
  message and check it in light and dark themes at a narrow and a normal
  window size, in demo mode (`cargo run --features demo -- --demo`).
- **Backend: it works, keep it that way.** `src/backend.rs`, `src/player.rs`,
  `src/api/`, `src/auth.rs`, `src/credentials.rs`, `src/session_reads.rs`,
  `src/sink.rs` and the queue rules in `src/app.rs` work. Change them only
  when a task needs it, and keep such changes small and separate from UI
  commits.
- **Removing features.** When stripping a feature, remove its UI, settings,
  tests, docs and translations together, and keep settings files written by
  the older build loading without error.

## Product boundaries

- Keep Spotsie a small native Spotify client. Do not add a browser engine,
  telemetry, a hosted backend, or alternate sources for Spotify audio.
- Playback capabilities come from librespot. Do not advertise or implement a
  capability merely because its name appears in a protobuf or enum. In
  particular, do not pursue Spotify Lossless or DRM circumvention.
- Do not broaden a task into adjacent features or a general refactor.

## Architecture

- `src/ui/` draws views and emits `Action`s. Apply actions after drawing in
  `src/app.rs`; do not mutate application state from inside a borrowed view.
- Network and playback work belongs on the runtime in `src/backend.rs` or in
  the player engine in `src/player.rs`, never as blocking work on the UI
  thread.
- Keep platform integrations behind target-specific modules or `cfg` blocks.
  A fix for one platform must keep the other two targets compiling.
- Settings and state files must remain readable, backward compatible, and
  atomically written. Never log credentials or authorization responses.
- Prefer existing dependencies. Explain any new crate in `Cargo.toml` next to
  the dependency when the reason is not obvious.
- egui, winit, librespot and the fastframe crates come from crmne's forks
  and tags, pinned in `Cargo.toml` (see `[patch.crates-io]`). Move all crates
  of one source to a new revision together.
- The egui fork shapes right-to-left runs in their own direction but leaves
  them in logical order. Pass logical text to `crate::bidi`, which reorders
  the laid-out runs; never reorder the string before layout.

Read `docs/_reference/how-it-connects.md` before changing authentication,
Spotify requests, Connect, credential storage, or network behaviour. Read
`docs/_reference/queue.md` before touching the queue: its rules are the
contract, and the queue tests in `src/app.rs` enforce them. Read the
nearby module tests before changing a state machine or API fallback.

`docs/_reference/what-spotify-allows.md` lists what the Web API, the
librespot session, and librespot playback each offer, and the requests
none of them can serve. Check it before building a Spotify-facing feature.

The interface is optimistic, always. A control shows its result the
moment it is used: a double-clicked song is the playing song, Next pops
the queue's head, an added song has its row. The backend then makes it
true and Spotify's state catches up behind; an answer that still tells
the story from before the user's action is stale, so hold the shown
state and ask again rather than let the lagging answer undo what the
user just did. Nothing the user did may ever flicker away and come back.

The update checker (`src/updates.rs`) points at this repository's GitHub
releases. Its Settings switches are hidden for now; only About's **Check for
updates** button uses it. Keep it working so the switches can return.

## Look and motion

Spotsie follows the look and feel of Magpie (the maintainer's other egui
app): calm, subtle, and never instant. Every animation goes through
`src/motion.rs`, with its durations (`MICRO` hovers, `STANDARD` toggles and
panels, `EMPHASIS` entrances, `GLIDE` values) and ease-out curves, and
requests repaints only while something moves. Hovers fade, choices ease
in, panels slide (`ui::sliding_panel`), pages rise into place, dialogs and
toasts land, and themes crossfade. Menus of alternatives put their check at
the right (`widgets::menu_choice`); dropdowns use `widgets::dropdown` and
`widgets::option`. Motion is off on test threads and for `--demo-shot`
(`motion::set_enabled`), so tests and screenshots see settled frames.

## Identity

Spotsie has its own identity, so it can be installed beside Spotifast
without sharing anything: app ID `io.github.ahaan_shah.Spotsie` (keyring service,
desktop file, Flatpak, macOS bundle), command and single-instance name
`spotsie`, and its own config, state and cache directories. Never reuse a
Spotifast identifier. `tests/branding.rs` fails if that name appears anywhere
but Markdown.

## Branches

Work on `main`, one topic per commit, each compiling and passing the checks
on its own. Keep `main` linear. Never rewrite
published history without explicit approval.

## Builds

Plain `cargo` is fine. Never put build output or large scratch files in
`/tmp`.

## Definition of done

- Add focused regression tests for changed behaviour. Use the `demo` feature
  for deterministic UI coverage and screenshots.
- Update the README and docs when user-visible behaviour, settings, files, or
  network access changes.
- Run the checks from `CONTRIBUTING.md`. Do not weaken a lint, delete a test,
  or add an `allow` merely to make CI green without explaining why the
  underlying rule does not apply.
- Report platform coverage honestly. Do not claim a platform was tested when
  it was only compiled or reasoned about.

## Installing and releasing

`scripts/install.sh` builds a release binary and installs it for the user
(`~/.local/bin`, the launcher entry and the icon). Run it after changes the
user wants on their system.

Versions are tagged `vMAJOR.MINOR.PATCH`, starting at v0.1.0:

1. Write `packaging/release-notes/vVERSION.md`: a short plain-language
   summary, then `New`, `Changed`, `Fixed` or `Removed` sections as they
   apply, each item leading with a bold user-facing result. Never write
   placeholder notes.
2. With everything else committed, run `scripts/release.sh VERSION`. It sets
   the version in `Cargo.toml`, `Cargo.lock` and the Flatpak metainfo,
   commits, and creates the annotated tag. It does not push.
3. Push only when the user asks: `git push && git push origin vVERSION`.
4. Run `scripts/install.sh` so the installed app is the new version.

CI runs on Linux only. The release workflow, which builds packages for every
platform, runs only when started by hand from the Actions tab on a tag.

The icon is `packaging/icons/spotsie.svg`; the app renders it at runtime.
After changing it, regenerate `packaging/macos/icon-1024.png` and
`packaging/windows/spotsie.ico` from it.

Never use em dashes in user-facing text. Use a full stop, comma, colon, or
parentheses instead.
