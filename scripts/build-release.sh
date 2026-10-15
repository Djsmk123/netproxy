#!/bin/bash
# NetProxy Release Build Script
# Builds both Rust daemon and SwiftUI app for testing

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  NetProxy Release Build Script${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Configuration
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build/release"
DAEMON_SOURCE="$PROJECT_ROOT/target/release/netproxyd"
APP_SOURCE="$PROJECT_ROOT/app/.build/release/NetProxy"

echo -e "${YELLOW}Project root:${NC} $PROJECT_ROOT"
echo -e "${YELLOW}Build output:${NC} $BUILD_DIR"
echo ""

# Clean previous build
if [ -d "$BUILD_DIR" ]; then
    echo -e "${YELLOW}Cleaning previous build...${NC}"
    rm -rf "$BUILD_DIR"
fi

mkdir -p "$BUILD_DIR"

# ============================================================================
# Step 1: Build Rust Daemon
# ============================================================================

echo -e "${GREEN}━━━ Step 1/3: Building Rust Daemon ━━━${NC}"
echo ""

cd "$PROJECT_ROOT"
echo "Running: cargo build --release --bin netproxyd"
echo ""

if cargo build --release --bin netproxyd; then
    echo ""
    echo -e "${GREEN}✓ Rust daemon built successfully${NC}"

    # Check binary size
    DAEMON_SIZE=$(du -h "$DAEMON_SOURCE" | cut -f1)
    echo -e "  Binary size: ${DAEMON_SIZE}"
else
    echo -e "${RED}✗ Rust build failed${NC}"
    exit 1
fi

echo ""

# ============================================================================
# Step 2: Build SwiftUI App
# ============================================================================

echo -e "${GREEN}━━━ Step 2/3: Building SwiftUI App ━━━${NC}"
echo ""

cd "$PROJECT_ROOT/app"

# Check if we have a Package.swift
if [ -f "Package.swift" ]; then
    echo "Building Swift package..."
    echo ""

    # Build with Swift Package Manager
    if swift build -c release; then
        echo ""
        echo -e "${GREEN}✓ Swift app built successfully${NC}"
    else
        echo ""
        echo -e "${YELLOW}⚠ Swift build had issues (continuing anyway)${NC}"
    fi
else
    echo -e "${YELLOW}⚠ No Package.swift found - skipping Swift build${NC}"
    echo "  (App will be built on demand)"
fi

echo ""

# ============================================================================
# Step 3: Package Release
# ============================================================================

echo -e "${GREEN}━━━ Step 3/3: Packaging Release ━━━${NC}"
echo ""

# Copy daemon
echo "Copying netproxyd..."
cp "$DAEMON_SOURCE" "$BUILD_DIR/netproxyd"
chmod +x "$BUILD_DIR/netproxyd"

# Check if Swift binary exists
if [ -f "$APP_SOURCE" ]; then
    echo "Copying NetProxy app..."
    cp "$APP_SOURCE" "$BUILD_DIR/NetProxy"
    chmod +x "$BUILD_DIR/NetProxy"
else
    echo -e "${YELLOW}Note: Swift app binary not found at expected location${NC}"
    echo "  You may need to build it separately"
fi

# Create example configuration
echo ""
echo "Creating example configuration..."

cat > "$BUILD_DIR/mocks.example.yaml" << 'YAML'
# NetProxy Mock Configuration Example
# Copy this to mocks.yaml and customize

version: 1
fixtures_dir: ./fixtures

rules:
  # Example 1: Mock API endpoint
  - id: example-api
    priority: 100
    enabled: true
    description: "Example API mock"
    match:
      method: GET
      host: "api.example.com"
      path: "/hello"
    action:
      type: respond
      status: 200
      headers:
        content-type: text/plain
      body:
        inline: "Hello from NetProxy!"
YAML

# Create README
cat > "$BUILD_DIR/README.txt" << 'README'
NetProxy Release Build
======================

This directory contains a release build of NetProxy for testing.

Contents:
---------
- netproxyd       : Proxy daemon (Rust binary)
- NetProxy        : macOS UI app (Swift binary, if built)
- mocks.example.yaml : Example mock configuration

Quick Start:
------------
1. Create a data directory:
   mkdir -p ~/NetProxy-Data/fixtures

2. Copy the example config:
   cp mocks.example.yaml ~/NetProxy-Data/mocks.yaml

3. Start the daemon:
   ./netproxyd --data-dir ~/NetProxy-Data

4. Test with curl:
   curl -x http://127.0.0.1:8899 http://api.example.com/hello

Documentation:
--------------
See the main repository for complete documentation:
https://github.com/Djsmk123/netproxy

Version Information:
--------------------
Milestone: M3 + M4 Phase 1 (Dynamic Registry)
Build Date: $(date)
README

# Create version info
cat > "$BUILD_DIR/VERSION" << VERSION
NetProxy Release Build
Milestone: M3 + M4 Phase 1
Build Date: $(date)
Git Commit: $(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
VERSION

echo -e "${GREEN}✓ Release packaged${NC}"

# ============================================================================
# Summary
# ============================================================================

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Release Build Complete!${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}Build location:${NC}"
echo "  $BUILD_DIR"
echo ""
echo -e "${YELLOW}Contents:${NC}"
ls -lh "$BUILD_DIR" | tail -n +2 | awk '{printf "  %-20s %5s  %s\n", $9, $5, ""}'
echo ""
echo -e "${YELLOW}Quick test:${NC}"
echo "  cd $BUILD_DIR"
echo "  ./netproxyd --help"
echo ""
echo -e "${YELLOW}To run with test config:${NC}"
echo "  mkdir -p ~/NetProxy-Test"
echo "  cp $BUILD_DIR/mocks.example.yaml ~/NetProxy-Test/mocks.yaml"
echo "  $BUILD_DIR/netproxyd --data-dir ~/NetProxy-Test"
echo ""
