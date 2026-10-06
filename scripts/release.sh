#!/usr/bin/env bash
# Records a new Spotsie version: sets it in Cargo.toml and Cargo.lock, adds it
# to the Flatpak metainfo, commits, and tags it vVERSION. Nothing is pushed.
#
#   scripts/release.sh 0.1.1
#
# Write packaging/release-notes/vVERSION.md first: it becomes the commit's
# and the tag's description. Then push with
#
#   git push && git push origin vVERSION
#
# and run scripts/install.sh to update the copy on this computer.
set -euo pipefail
cd "$(dirname "$0")/.."

version="${1:-}"
if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]]; then
    echo "usage: scripts/release.sh VERSION   (for example 0.1.1)" >&2
    exit 2
fi
tag="v$version"
notes="packaging/release-notes/$tag.md"

if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    echo "$tag already exists." >&2
    exit 1
fi
if [[ ! -s "$notes" ]]; then
    echo "Write $notes first: what changed in $tag, for the tag and the release." >&2
    exit 1
fi
if [[ -n "$(git status --porcelain -- . ":!$notes")" ]]; then
    echo "Commit or set aside other changes first; the release commit holds only the version." >&2
    exit 1
fi

# The package's own version line, the first in the file.
sed -i "0,/^version = \".*\"/s//version = \"$version\"/" Cargo.toml
cargo update --workspace --offline --quiet

metainfo=packaging/flatpak/io.github.ahaan_shah.Spotsie.metainfo.xml
python3 - "$metainfo" "$version" "$(date +%F)" <<'EOF'
import sys
path, version, date = sys.argv[1:]
text = open(path).read()
entry = (f'    <release version="{version}" date="{date}">\n'
         f'      <url>https://github.com/ahaan-shah/spotsie/releases/tag/v{version}</url>\n'
         f'    </release>\n')
marker = "  <releases>\n"
if f'version="{version}"' not in text:
    text = text.replace(marker, marker + entry, 1)
open(path, "w").write(text)
EOF

git add Cargo.toml Cargo.lock "$metainfo" "$notes"
git commit -q -F - <<EOF
Release $tag

$(cat "$notes")
EOF
git tag -a "$tag" -F "$notes"

echo "Tagged $tag. Push with: git push && git push origin $tag"
