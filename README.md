# Spotsie

**Spotify, native and light, the way Spotify looks.** Spotsie is a Spotify
client written in Rust with [egui](https://github.com/emilk/egui). It plays
music through [librespot](https://github.com/librespot-org/librespot), has no
browser engine, and runs on Linux, macOS, and Windows.

Spotsie is built on the backend of
[Spotifast](https://github.com/crmne/spotifast) by Carmine Paolino and
has its own interface, modelled on Spotify's desktop app.

**Playback needs Spotify Premium.** Free accounts can browse and search, but
cannot play music through Spotsie.

## Status

Early. Version 0.1.0 is Spotifast 0.12.0's code under Spotsie's own name,
without the Winamp mini player and the MilkDrop visualiser. The interface
rework comes next.

Spotsie has its own app ID (`io.github.ahaan_shah.Spotsie`), command,
settings, sign-in and data folders, so it runs beside an installed Spotifast
without sharing anything. Spotsie never checks for updates on its own.

## Build and run

Rust 1.98 or newer. On Arch Linux:

```sh
sudo pacman -S --needed base-devel alsa-lib libpulse libxkbcommon wayland
cargo run --release
```

To look at the interface without a Spotify account:

```sh
cargo run --features demo -- --demo
```

Other systems: see [Build from source](docs/_guide/getting-started.md#build-from-source).

## Install on this computer

```sh
scripts/install.sh              # build and install, or update an installed copy
scripts/install.sh --uninstall  # remove it (settings and sign-in stay)
```

This puts `spotsie` in `~/.local/bin` and adds Spotsie to the app launcher.

## Versions

Releases are tagged `v0.1.0`, `v0.1.1` and so on. To cut one, write
`packaging/release-notes/vVERSION.md`, then run `scripts/release.sh VERSION`.
It sets the version, commits and tags. Then push with
`git push && git push origin vVERSION` and run `scripts/install.sh`.

## Docs

- [Getting started](docs/_guide/getting-started.md): sign-in, playback on this computer, themes, fonts, proxies
- [Everyday use](docs/_guide/using-spotsie.md): keyboard shortcuts, command-line control
- [Settings and files](docs/_reference/settings-and-files.md) and [Privacy](docs/_reference/privacy.md)
- [How it connects](docs/_reference/how-it-connects.md) and [What Spotify allows](docs/_reference/what-spotify-allows.md)

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) and, for coding agents,
[AGENTS.md](AGENTS.md). Translations live in `assets/i18n/`; see
[Translating](docs/_reference/translating.md). Release packaging is described
in [docs/_reference/packaging.md](docs/_reference/packaging.md).

## Acknowledgements

Spotsie starts from [Spotifast](https://github.com/crmne/spotifast)
(MIT, © Carmine Paolino), and uses
[librespot](https://github.com/librespot-org/librespot),
[egui](https://github.com/emilk/egui), the [Inter](https://rsms.me/inter/)
typeface (OFL), and [Lucide](https://lucide.dev) icons (ISC).

Spotsie is an independent project and is not affiliated with Spotify.
Spotify is a trademark of Spotify AB.

Licensed under the [MIT License](LICENSE).
