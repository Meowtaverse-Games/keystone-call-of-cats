#!/bin/bash
set -e

# Default configuration
ITCH_USER="${ITCH_USER:-meowtaverse-games}"
ITCH_GAME="${ITCH_GAME:-keystone-call-of-cats}"
CHANNEL="mac-universal"
ENABLE_STEAM=false
BUILD_SCRIPT="./scripts/build_macos.sh"
SKIP_BUILD=false

# Simple arg parsing
for arg in "$@"; do
  case $arg in
    --skip-build)
      SKIP_BUILD=true
      shift
      ;;
  esac
done

# Check for butler
if ! command -v butler &> /dev/null; then
    echo "Error: butler is not installed or not in PATH."
    echo "Please install it from https://itch.io/docs/butler/installing.html"
    exit 1
fi

# Build
if [ "$SKIP_BUILD" = false ]; then
    echo "Running build script..."
    if [ -f "$BUILD_SCRIPT" ]; then
        echo "$BUILD_SCRIPT"
        "$BUILD_SCRIPT"
    else
        echo "Error: Build script $BUILD_SCRIPT not found."
        exit 1
    fi
else
    echo "Skipping build..."
fi

APP_DIR="target/release/KeystoneCC.app"

if [ ! -d "$APP_DIR" ]; then
    echo "Error: App bundle $APP_DIR not found. Did the build fail?"
    exit 1
fi

echo "Pushing to itch.io..."
echo "Target: $ITCH_USER/$ITCH_GAME:$CHANNEL"

butler push "$APP_DIR" "$ITCH_USER/$ITCH_GAME:$CHANNEL"

echo "Done."
