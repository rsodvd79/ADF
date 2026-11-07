#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
test "$(uname -s)" = "Darwin"
test "$(uname -m)" = "arm64"
test -f target/release/adf
mkdir -p target/bundle/ADF.app/Contents/MacOS target/bundle/ADF.app/Contents/Resources/examples
cp target/release/adf target/bundle/ADF.app/Contents/MacOS/adf
cp packaging/macos/Info.plist target/bundle/ADF.app/Contents/Info.plist
cp examples/ciao.xml examples/showcase.xml target/bundle/ADF.app/Contents/Resources/examples/
cp README.md target/bundle/ADF.app/Contents/Resources/
plutil -lint target/bundle/ADF.app/Contents/Info.plist
ditto -c -k --sequesterRsrc --keepParent target/bundle/ADF.app target/bundle/adf-macos-arm64.zip
