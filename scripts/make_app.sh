#!/bin/sh
# Builds dist/Prtl2Mac.app, a native macOS app bundle of the game.
# Usage (from the repo): sh scripts/make_app.sh
# The previous app is kept as dist/Prtl2Mac-previous.app; the build id
# (version+commit[-dirty], UTC time) is in Contents/Resources/BUILD.txt and
# the bundle's CFBundleVersion.
# The app finds Portal 2 by itself (Steam libraries, or a depot downloaded
# with the Steam console); PORTAL2_DIR still overrides it when set.
set -e
cd "$(dirname "$0")/.."
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
"$CARGO" build --release -p p2_game
VERSION=$(grep -m1 '^version' crates/p2_game/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
COMMIT=$(git rev-parse --short HEAD)
[ -n "$(git status --porcelain --untracked-files=no)" ] && COMMIT="$COMMIT-dirty"
STAMP=$(date -u +%Y%m%dT%H%MZ)
BUILD="$VERSION+$COMMIT.$STAMP"
FINAL=dist/Prtl2Mac.app
APP=dist/Prtl2Mac.app.new
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/portal2 "$APP/Contents/MacOS/portal2"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Prtl2Mac</string>
  <key>CFBundleDisplayName</key><string>Prtl2Mac</string>
  <key>CFBundleIdentifier</key><string>com.byaydenrx.prtl2mac</string>
  <key>CFBundleExecutable</key><string>portal2</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${BUILD}</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.games</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSArchitecturePriority</key><array><string>arm64</string></array>
</dict>
</plist>
PLIST
printf '%s\n' "$BUILD" > "$APP/Contents/Resources/BUILD.txt"
# Ad-hoc signature so macOS runs the bundle as a whole.
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || echo "note: codesign failed; the app may need right-click > Open"
if [ -d "$FINAL" ]; then
    rm -rf dist/Prtl2Mac-previous.app
    mv "$FINAL" dist/Prtl2Mac-previous.app
fi
mv "$APP" "$FINAL"
echo "built $FINAL ($BUILD); previous build in dist/Prtl2Mac-previous.app"
