---
title: Release packaging
description: How a tagged release turns into downloadable packages.
nav_order: 20
---

Spotsie keeps its release asset definitions and nFPM configuration in
`native-packages.yaml` and `packaging/`. Common automation comes from the
pinned [native-packages](https://github.com/crmne/native-packages) gem,
installed with `gem install native-packages --version 0.8.1`.

Pushing a `v*` tag runs `.github/workflows/release.yml`, which builds the
Linux, macOS and Windows artifacts, the Flatpak bundle and the macOS DMG, then
writes and signs `checksums.txt`. The release body comes from
`packaging/release-notes/vVERSION.md`, which must exist before the tag is
pushed.

Nothing is published to Homebrew, the AUR or Flathub. The Arch templates in
`packaging/arch/` are kept for building local packages.

The application retains its Flatpak manifests, macOS bundle/signing
configuration and Windows installer configuration. nFPM does not replace
these platform tools. Complete Apple CI secrets enable signing, notarization
and ticket validation automatically before checksums are written; without
them the macOS build is unsigned.
