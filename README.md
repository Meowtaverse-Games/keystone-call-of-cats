# Keystone: Call of Cats
Keystone: Call of Cats invites you to guide curious cats and stones toward a shared goal while discovering how to program along the way.

## Repository Layout
- `src/main.rs` – Binary entrypoint that configures Bevy and registers the game layers
- `src/config.rs` – Game configuration constants and settings
- `src/plugins/` – Modular game features and engine integrations
- `src/resources/` – Shared game state and data structures
- `src/scenes/` – High-level game states (e.g., Title, Gameplay) and screen definitions
- `src/systems/` – Game logic systems (input handling, movement, collision, etc.)
- `src/util/` – Helper functions and common utilities
- `assets/` – Runtime assets (images, audio, fonts, stage data)
- `scripts/` – Build, packaging, and deployment scripts
- `tools/` – Internal development tools (e.g., sprite sheet exporter)
- `ext-assets/` – Source files gathered from external tools/artists before import or optimization

## How To Run

- Prerequisites: Rust toolchain (stable) via `rustup`.
- Run (debug): `cargo run`
- Run (optimized): `cargo run --release`

Notes:
- Transparent window behavior is primarily tuned for macOS/Linux. On Windows, transparency or reveal effects may differ.
- Typing prints characters; Backspace and Enter are handled. Window resize adjusts layout.

## ChromeOS (Android APK)

`scripts/build-chromeos-apk` creates a debug-signed universal APK for Android-enabled Chromebooks. It uses Docker because the Android SDK/NDK command-line tools are x86_64 binaries; on this ARM64 build host Docker runs them through binfmt.

```sh
./scripts/build-chromeos-apk
```

The resulting files are `dist/chromeos/keystone-cc-debug.apk` and its SHA-256 checksum. The APK contains both `arm64-v8a` and `x86_64` native libraries and requires Android 12 (API 31) or newer.

To install it on a Chromebook, enable Android app development / ADB debugging in ChromeOS settings, connect with `adb`, then run:

```sh
adb install -r dist/chromeos/keystone-cc-debug.apk
```

Alternatively, transfer the APK to the Chromebook and open it with the Android package installer. This is a debug build only; it is not Play Store signed.

## Code License

- Scope: All source code outside of the `assets/` directory (`src/`, `scripts/`, build files, documentation, etc.)
- License: GNU General Public License v3.0 — see `LICENSE`

## Assets License

- Scope: All images, audio, fonts, and other media files inside the `assets/` directory
- License: Each subfolder of `assets/` contains its own `LICENSE` file, which governs that folder’s contents
