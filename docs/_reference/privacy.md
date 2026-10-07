---
title: Privacy
description: What Spotsie stores on your computer, what it sends and to whom, and what it never collects.
nav_order: 4
---

Spotsie is a desktop app that runs entirely on your computer. It has no
account of its own, no server, no telemetry, no analytics, and no advertising.
Its author receives nothing about you or how you use it.

This page covers the Spotsie app, version 0.8.0 and later. Earlier versions
kept sign-ins in files instead of the system credential store; update to a
current release.

## What stays on your computer

- **Spotify sign-ins.** The grants Spotify issues when you sign in, and the
  reusable playback credential, are kept in the system credential store:
  Credential Manager on Windows, Keychain on macOS, and Secret Service on
  Linux. Your Spotify password never passes through Spotsie; you sign in
  on Spotify's own pages. A proxy password, if you set one, uses the same
  store.
- **Settings and history.** Settings, window positions, recent plays, the
  last session and themes live in the config directory.
- **Caches.** Downloaded audio, artwork, lyrics and library metadata live in
  the cache directory and can be deleted at any time.
- **Log.** `spotsie.log` records errors and diagnostics. It stays on your
  computer and never contains credentials; share it only if you choose to
  attach it to a bug report.

[Settings & Files](settings-and-files.md) lists every location and what is safe
to delete. **Sign out** in Settings removes the stored credentials.

## What is sent, and to whom

Spotsie connects only to the services below.

- **Spotify.** Sign-in, your library, search, playlists, playback and Spotify
  Connect all go to Spotify, under your account. Spotify's own
  [privacy policy](https://www.spotify.com/legal/privacy-policy/) applies to
  that data.
- **LRCLIB.** When the lyrics are open and Spotify has no lyrics for the
  song, Spotsie sends its artist, title, album and length to
  [lrclib.net](https://lrclib.net). Nothing identifying you is included.
- **GitHub.** When you press **Check for updates** in Settings, Spotsie asks
  GitHub for the latest release. Downloading an update also fetches files
  from GitHub. No Spotify data is sent.
- **Your local network.** Spotsie looks for Spotify Connect speakers over
  mDNS and talks to the ones you choose.

Links you open from the app open in your browser.

## Analytics

Spotsie contains no analytics or telemetry.

## Questions

Ask on [GitHub](https://github.com/ahaan-shah/spotsie/issues).
