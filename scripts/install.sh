#!/usr/bin/env bash
# Builds Spotsie in release mode and installs it for the current user, so it
# shows up in the app launcher like any other app. Run it again after pulling
# or making changes to update the installed copy; `--uninstall` removes it.
#
# Installs into ~/.local (no root needed):
#   ~/.local/bin/spotsie
#   ~/.local/share/applications/spotsie.desktop
#   ~/.local/share/icons/hicolor/scalable/apps/spotsie.svg
#
# Settings, sign-in and caches live elsewhere (~/.config/spotsie,
# ~/.local/state/spotsie, ~/.cache/spotsie and the system keyring) and are
# never touched here.
set -euo pipefail
cd "$(dirname "$0")/.."

bin_dir="${XDG_BIN_HOME:-$HOME/.local/bin}"
data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"
binary="$bin_dir/spotsie"
desktop="$data_dir/applications/spotsie.desktop"
icon="$data_dir/icons/hicolor/scalable/apps/spotsie.svg"

refresh_caches() {
    if command -v update-desktop-database >/dev/null; then
        update-desktop-database -q "$data_dir/applications" || true
    fi
    if command -v gtk-update-icon-cache >/dev/null; then
        gtk-update-icon-cache -q -t "$data_dir/icons/hicolor" || true
    fi
}

if [[ "${1:-}" == "--uninstall" ]]; then
    rm -f "$binary" "$desktop" "$icon"
    refresh_caches
    echo "Removed Spotsie. Settings and sign-in were left in place."
    exit 0
fi

cargo build --release --locked

# A running copy keeps the old binary open; replacing the file is safe, and
# the new version starts the next time Spotsie is opened.
install -Dm755 target/release/spotsie "$binary"
install -Dm644 packaging/icons/spotsie.svg "$icon"
# The launcher's environment may not have ~/.local/bin on PATH, so the
# installed entry points at the binary directly.
mkdir -p "$(dirname "$desktop")"
sed "s|^Exec=spotsie |Exec=$binary |" packaging/applications/spotsie.desktop >"$desktop"
chmod 644 "$desktop"
refresh_caches

echo "Installed Spotsie $("$binary" --version | awk '{print $2}') to $binary"
