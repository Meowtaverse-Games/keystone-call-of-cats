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

## Web build (experimental)

The game can be bundled for browsers as a WebAssembly build. Install the
`wasm32-unknown-unknown` Rust target and [Trunk](https://trunkrs.dev/) once:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.21.14 --locked
```

Build the distributable files (HTML, JavaScript glue, WebAssembly, and runtime
assets) with:

```sh
./scripts/build_web.sh
```

The resulting `dist/` directory is intentionally ignored by Git. Serve it over
HTTP rather than opening `index.html` directly, for example:

```sh
trunk serve --cargo-profile web-release
```

Known web limitations:

- Saves are held in memory only and are lost when the page reloads; browser
  persistent storage is not implemented yet.
- Steam integration is not supported by the web build.
- Script execution currently uses native worker threads, so running user
  scripts in the browser is not supported yet.
- The WebAssembly build intentionally includes only the Bevy renderer, window,
  and audio features used by this 2D game; desktop builds retain their existing
  full Bevy feature set.

Notes:
- Transparent window behavior is primarily tuned for macOS/Linux. On Windows, transparency or reveal effects may differ.
- Typing prints characters; Backspace and Enter are handled. Window resize adjusts layout.

## Code License

- Scope: All source code outside of the `assets/` directory (`src/`, `scripts/`, build files, documentation, etc.)
- License: GNU General Public License v3.0 — see `LICENSE`

## Assets License

- Scope: All images, audio, fonts, and other media files inside the `assets/` directory
- License: Each subfolder of `assets/` contains its own `LICENSE` file, which governs that folder’s contents
