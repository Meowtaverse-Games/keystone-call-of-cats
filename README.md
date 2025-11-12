# Keystone: Call of Cats
Keystone: Call of Cats invites you to guide curious cats and stones toward a shared goal while discovering how to program along the way.

## Repository Layout
- `src/main.rs` – Binary entrypoint that configures Bevy and registers the game layers
- `src/application/` – High-level game/application state, modes, and platform glue (e.g., Steam)
- `src/domain/` – Engine-agnostic rules, chunk grammars, and script definitions shared across layers
- `src/infrastructure/` – Bridges to external systems and engine services (Steam APIs, tiled import, scripting, asset loading)
- `src/presentation/` – Scenes, cameras, and UI/state presentation built on Bevy ECS
- `assets/` – Runtime assets loaded by the game
- `ext-assets/` – Source files gathered from external tools/artists before import or optimization

## How To Run

- Prerequisites: Rust toolchain (stable) via `rustup`.
- Run (debug): `cargo run`
- Run (optimized): `cargo run --release`

Notes:
- Transparent window behavior is primarily tuned for macOS/Linux. On Windows, transparency or reveal effects may differ.
- Typing prints characters; Backspace and Enter are handled. Window resize adjusts layout.

## Code License

- Scope: All source code outside of the `assets/` directory (`src/`, `scripts/`, build files, documentation, etc.)
- License: GNU General Public License v3.0 — see `LICENSE`

## Assets License

- Scope: All images, audio, fonts, and other media files inside the `assets/` directory
- License: Each subfolder of `assets/` contains its own `LICENSE` file, which governs that folder’s contents
