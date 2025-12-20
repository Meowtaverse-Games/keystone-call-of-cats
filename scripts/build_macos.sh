#!/bin/bash
set -e

# Mimic the environment variable from the workflow
export CARGO_TERM_COLOR=always

# Default to enabling Steam if not specified
if [ -z "$ENABLE_STEAM" ]; then
    ENABLE_STEAM=true
fi

echo "Building with ENABLE_STEAM=$ENABLE_STEAM"

if [ "$ENABLE_STEAM" = "true" ]; then
    CARGO_FLAGS="--release"
else
    CARGO_FLAGS="--release --no-default-features"
fi

# Run the build command
echo cargo build $CARGO_FLAGS
cargo build $CARGO_FLAGS

APP_NAME="KeystoneCC.app"
OUTPUT_DIR="target/release/$APP_NAME"
CONTENTS_DIR="$OUTPUT_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

echo "Creating app bundle structure..."
rm -rf "$OUTPUT_DIR"
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# Create Info.plist
cat > "$CONTENTS_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>keystone-cc</string>
    <key>CFBundleIdentifier</key>
    <string>com.meowtaverse-games.keystone-cc</string>
    <key>CFBundleName</key>
    <string>keystone: call of cats</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
</dict>
</plist>
EOF

# Copy executable as binary
cp target/release/keystone-cc "$MACOS_DIR/keystone-cc-bin"

# Create wrapper script
cat > "$MACOS_DIR/keystone-cc" <<EOF
#!/bin/bash
DIR="\$(cd "\$(dirname "\$0")" && pwd)"
cd "\$DIR"
exec "\$DIR/keystone-cc-bin" "\$@"
EOF
chmod +x "$MACOS_DIR/keystone-cc"

# Copy the steam library to the output directory only if Steam is enabled
if [ "$ENABLE_STEAM" = "true" ]; then
    find target/release/build -name "libsteam_api.dylib" -exec cp {} "$MACOS_DIR/" \;
fi

# Copy assets to the output directory (dereference symlinks)
# Bevy looks for assets relative to executable on macOS by default in this configuration
cp -RL assets "$MACOS_DIR/"

echo "Build complete."
echo "App Bundle location: $OUTPUT_DIR"
