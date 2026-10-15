#!/bin/bash
# Package NetProxy as a macOS .app bundle for distribution

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  NetProxy App Bundle Packager${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="NetProxy"
APP_BUNDLE="$PROJECT_ROOT/build/$APP_NAME.app"
DAEMON_BIN="$PROJECT_ROOT/target/release/netproxyd"

echo -e "${YELLOW}Creating app bundle structure...${NC}"

# Clean and create bundle structure
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# Create Info.plist
cat > "$APP_BUNDLE/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>netproxyd</string>
    <key>CFBundleIdentifier</key>
    <string>dev.netproxy.app</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>NetProxy</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.3.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

# Check if daemon exists
if [ ! -f "$DAEMON_BIN" ]; then
    echo -e "${YELLOW}Building Rust daemon first...${NC}"
    cd "$PROJECT_ROOT"
    cargo build --release --bin netproxyd
fi

# Copy daemon binary
echo -e "${YELLOW}Copying daemon binary...${NC}"
cp "$DAEMON_BIN" "$APP_BUNDLE/Contents/MacOS/netproxyd"
chmod +x "$APP_BUNDLE/Contents/MacOS/netproxyd"

# Create launch script (optional, if we want a wrapper)
cat > "$APP_BUNDLE/Contents/MacOS/launch.sh" << 'LAUNCH'
#!/bin/bash
# Launch NetProxy daemon
DIR="$(cd "$(dirname "$0")" && pwd)"
DATA_DIR="$HOME/Library/Application Support/NetProxy"

mkdir -p "$DATA_DIR"
exec "$DIR/netproxyd" --data-dir "$DATA_DIR"
LAUNCH
chmod +x "$APP_BUNDLE/Contents/MacOS/launch.sh"

# Create README in Resources
cat > "$APP_BUNDLE/Contents/Resources/README.txt" << 'README'
NetProxy - HTTP/HTTPS Debugging Proxy
======================================

To run:
-------
1. Right-click NetProxy.app and choose "Open"
2. macOS will ask to confirm (first time only)
3. The daemon will start in the background

Data Directory:
---------------
~/Library/Application Support/NetProxy/

Configuration:
--------------
Create ~/Library/Application Support/NetProxy/mocks.yaml
to configure mock rules.

Proxy Settings:
---------------
HTTP Proxy:  127.0.0.1:8899
HTTPS Proxy: 127.0.0.1:8899

Documentation:
--------------
https://github.com/Djsmk123/netproxy
README

echo -e "${GREEN}✓ App bundle created${NC}"
echo ""
echo -e "${YELLOW}Location:${NC} $APP_BUNDLE"
echo ""

# Show size
APP_SIZE=$(du -sh "$APP_BUNDLE" | cut -f1)
echo -e "${YELLOW}Size:${NC} $APP_SIZE"
echo ""

# Create a DMG (optional)
if command -v hdiutil &> /dev/null; then
    echo -e "${YELLOW}Creating DMG...${NC}"
    DMG_PATH="$PROJECT_ROOT/build/$APP_NAME.dmg"
    rm -f "$DMG_PATH"

    hdiutil create -volname "$APP_NAME" \
        -srcfolder "$APP_BUNDLE" \
        -ov -format UDZO \
        "$DMG_PATH" 2>/dev/null

    if [ -f "$DMG_PATH" ]; then
        DMG_SIZE=$(du -sh "$DMG_PATH" | cut -f1)
        echo -e "${GREEN}✓ DMG created${NC}"
        echo -e "${YELLOW}Location:${NC} $DMG_PATH"
        echo -e "${YELLOW}Size:${NC} $DMG_SIZE"
    fi
fi

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Packaging complete!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}To test:${NC}"
echo "  open $APP_BUNDLE"
echo ""
