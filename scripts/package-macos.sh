#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
test "$(uname -s)" = "Darwin"
test "$(uname -m)" = "arm64"
test -f target/release/adf
timestamp="${BUILD_TIMESTAMP:-$(date -u +'%Y.%m.%d.%H.%M')}"
if [[ ! "$timestamp" =~ ^[0-9]{4}\.[0-9]{2}\.[0-9]{2}\.[0-9]{2}\.[0-9]{2}$ ]]; then
    echo "BUILD_TIMESTAMP must use yyyy.MM.dd.HH.mm" >&2
    exit 1
fi
mkdir -p target/bundle/ADF.app/Contents/MacOS target/bundle/ADF.app/Contents/Resources/examples
cp target/release/adf target/bundle/ADF.app/Contents/MacOS/adf
cp packaging/macos/Info.plist target/bundle/ADF.app/Contents/Info.plist
cp examples/ciao.xml examples/showcase.xml target/bundle/ADF.app/Contents/Resources/examples/
cp README.md target/bundle/ADF.app/Contents/Resources/
plutil -lint target/bundle/ADF.app/Contents/Info.plist
archive="target/bundle/ADF.macos.${timestamp}.zip"
ditto -c -k --sequesterRsrc --keepParent target/bundle/ADF.app "$archive"
echo "$archive"
