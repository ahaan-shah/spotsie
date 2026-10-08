#!/bin/sh
# Spotsie installer for Linux and macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/ahaan-shah/spotsie/main/install.sh | sh
#
# To uninstall (your settings and sign-in are kept):
#
#   curl -fsSL https://raw.githubusercontent.com/ahaan-shah/spotsie/main/install.sh | sh -s -- --uninstall
#
# Environment:
#   SPOTSIE_VERSION   install a specific version (e.g. 0.1.1) instead of the latest
#   SPOTSIE_BASE_URL  download from a mirror instead of GitHub releases (needs SPOTSIE_VERSION)
set -eu

REPO="ahaan-shah/spotsie"

say() { printf '\033[1;32mspotsie\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror\033[0m %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "this installer needs '$1'"; }

OS="$(uname -s)"
BIN_DIR="$HOME/.local/bin"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
# The program lives in its own folder with the marker that lets it update
# itself from Settings; ~/.local/bin only holds a link to it.
APP_DIR="$HOME/.local/lib/spotsie"
ICON="$DATA/icons/hicolor/scalable/apps/spotsie.svg"
DESKTOP="$DATA/applications/spotsie.desktop"

refresh() {
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database -q "$DATA/applications" || true
    command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q -t "$DATA/icons/hicolor" 2>/dev/null || true
}

# Removes exactly what an install put there. Settings, sign-in and caches
# stay, so reinstalling picks up where you left off.
uninstall() {
    case "$OS" in
    Linux)
        [ -e "$APP_DIR/spotsie" ] || [ -e "$DESKTOP" ] || die "Spotsie isn't installed in $APP_DIR"
        rm -rf "$APP_DIR"
        [ -L "$BIN_DIR/spotsie" ] && rm -f "$BIN_DIR/spotsie"
        rm -f "$DESKTOP" "$ICON"
        refresh
        say "uninstalled Spotsie"
        say "your settings are still in ~/.config/spotsie (delete it to remove them too)"
        ;;
    Darwin)
        found=""
        for app in /Applications/Spotsie.app "$HOME/Applications/Spotsie.app"; do
            [ -e "$app" ] || continue
            found=1
            rm -rf "$app" 2>/dev/null || die "couldn't remove $app (try: sudo rm -rf '$app')"
            say "uninstalled $app"
        done
        [ -n "$found" ] || die "Spotsie.app isn't in /Applications or ~/Applications"
        ;;
    *) die "unsupported OS: $OS (this script is for Linux and macOS)" ;;
    esac
}

case "${1:-}" in
"") ;;
--uninstall)
    uninstall
    exit 0
    ;;
*) die "unknown option: $1 (the only option is --uninstall)" ;;
esac

need curl
need uname

# Release files carry the version in their names, so find out which one is
# the latest: GitHub redirects /releases/latest to its tag.
if [ -n "${SPOTSIE_VERSION:-}" ]; then
    TAG="v${SPOTSIE_VERSION#v}"
else
    latest="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest")" ||
        die "couldn't reach GitHub"
    TAG="${latest##*/}"
    case "$TAG" in
    v[0-9]*) ;;
    *) die "couldn't find the latest release (set SPOTSIE_VERSION to pick one)" ;;
    esac
fi
BASE="${SPOTSIE_BASE_URL:-https://github.com/$REPO/releases/download/$TAG}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT INT TERM

fetch() {
    say "downloading $1"
    curl -fL --progress-bar "$BASE/$1" -o "$TMP/$1" || die "couldn't download $BASE/$1"
}

verify() {
    curl -fsL "$BASE/checksums.txt" -o "$TMP/checksums.txt" || die "couldn't download the release's checksums"
    expected="$(grep " $1\$" "$TMP/checksums.txt" | cut -d' ' -f1)"
    [ -n "$expected" ] || die "$1 isn't listed in the release's checksums"
    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "$TMP/$1" | cut -d' ' -f1)"
    else
        actual="$(shasum -a 256 "$TMP/$1" | cut -d' ' -f1)"
    fi
    [ "$expected" = "$actual" ] || die "checksum mismatch for $1"
    say "checksum ok"
}

ARCH="$(uname -m)"

case "$OS" in
Linux)
    case "$ARCH" in
    x86_64 | amd64) ARCH=x86_64 ;;
    aarch64 | arm64) ARCH=aarch64 ;;
    *) die "unsupported architecture: $ARCH (you can build from source: https://github.com/$REPO#building-from-source)" ;;
    esac
    need tar
    STEM="spotsie-$TAG-$ARCH-unknown-linux-gnu"
    fetch "$STEM.tar.gz"
    verify "$STEM.tar.gz"
    tar -xzf "$TMP/$STEM.tar.gz" -C "$TMP"
    SRC="$TMP/$STEM"

    mkdir -p "$APP_DIR" "$BIN_DIR" "$DATA/applications" "$(dirname "$ICON")"
    # Replace, never write into, a running copy's file.
    install -m 755 "$SRC/spotsie" "$APP_DIR/.spotsie.new"
    mv -f "$APP_DIR/.spotsie.new" "$APP_DIR/spotsie"
    install -m 644 "$SRC/spotsie-portable.txt" "$APP_DIR/spotsie-portable.txt"
    ln -sfn "$APP_DIR/spotsie" "$BIN_DIR/spotsie"
    install -m 644 "$SRC/packaging/icons/spotsie.svg" "$ICON"
    sed "s|^Exec=spotsie |Exec=$APP_DIR/spotsie |" "$SRC/packaging/applications/spotsie.desktop" >"$DESKTOP"
    chmod 644 "$DESKTOP"
    refresh

    say "installed Spotsie ${TAG#v} to $APP_DIR"
    case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) say "note: $BIN_DIR isn't on your PATH. Add it, or open Spotsie from your app menu" ;;
    esac
    say "run it with: spotsie"
    ;;
Darwin)
    need hdiutil
    DMG="spotsie-$TAG-macos-universal.dmg"
    fetch "$DMG"
    verify "$DMG"
    MOUNT="$TMP/mount"
    mkdir -p "$MOUNT"
    hdiutil attach -nobrowse -readonly -quiet -mountpoint "$MOUNT" "$TMP/$DMG" || die "couldn't open $DMG"
    if [ -w /Applications ]; then DEST=/Applications; else DEST="$HOME/Applications"; mkdir -p "$DEST"; fi
    rm -rf "$DEST/Spotsie.app"
    cp -R "$MOUNT/Spotsie.app" "$DEST/" || { hdiutil detach -quiet "$MOUNT"; die "couldn't copy Spotsie.app to $DEST"; }
    hdiutil detach -quiet "$MOUNT" || true
    # Downloaded with curl, so there is no quarantine flag; clear it just in case.
    xattr -dr com.apple.quarantine "$DEST/Spotsie.app" 2>/dev/null || true
    say "installed $DEST/Spotsie.app"
    say "open it from Launchpad, or run: open -a Spotsie"
    ;;
*)
    die "unsupported OS: $OS (on Windows, use install.ps1)"
    ;;
esac
