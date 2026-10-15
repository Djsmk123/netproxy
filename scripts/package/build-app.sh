#!/bin/zsh
# Builds NetProxy.app (SwiftUI shell + embedded Rust helper), signs inside-out with hardened runtime. Usage: build-app.sh <outdir> [identity]
set -e
HERE=${0:A:h}; ROOT=${HERE:h:h}; OUT=${1:?outdir}; ID=${2:-$(security find-identity -v -p codesigning | sed -n 's/.*"\(.*\)"/\1/p' | head -1)}
APP=$OUT/NetProxy.app; rm -rf $APP; mkdir -p $APP/Contents/MacOS $APP/Contents/Helpers
swiftc -parse-as-library -O -target arm64-apple-macosx14.0 $HERE/App.swift -o $APP/Contents/MacOS/NetProxy
cp $ROOT/target/release/np-spike-rust $APP/Contents/Helpers/netproxyd
cat > $APP/Contents/Info.plist <<'XML'
<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>dev.netproxy.m0</string><key>CFBundleName</key><string>NetProxy</string>
<key>CFBundleExecutable</key><string>NetProxy</string><key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.0.1</string><key>CFBundleVersion</key><string>1</string>
<key>LSMinimumSystemVersion</key><string>14.0</string><key>NSHighResolutionCapable</key><true/>
</dict></plist>
XML
echo "signing with: $ID"
codesign --force --options runtime --timestamp=none -s "$ID" $APP/Contents/Helpers/netproxyd
codesign --force --options runtime --timestamp=none -s "$ID" $APP
codesign --verify --deep --strict --verbose=2 $APP 2>&1 | tail -2
codesign -dv $APP 2>&1 | grep -E "flags|Identifier"
