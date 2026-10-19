#!/bin/sh
# Builds dist/Prtl2Mac.app, a native macOS app bundle of the game.
# Usage (from the repo): sh scripts/make_app.sh
# The app finds Portal 2 by itself (Steam libraries, or a depot downloaded
# with the Steam console); PORTAL2_DIR still overrides it when set.
set -e
cd "$(dirname "$0")/.."
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
"$CARGO" build --release -p p2_game
VERSION=$(grep -m1 '^version' crates/p2_game/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
APP=dist/Prtl2Mac.app
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
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.games</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSArchitecturePriority</key><array><string>arm64</string></array>
</dict>
</plist>
PLIST
# Ad-hoc signature so macOS runs the bundle as a whole.
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || echo "note: codesign failed; the app may need right-click > Open"
echo "built $APP"
