# Keystone: Call of Cats
Keystone: Call of Cats invites you to guide curious cats and stones toward a shared goal while discovering how to program along the way.

## Repository Layout
- `app/` – Binary crate `keystone-cc` that bootstraps Bevy and wires all other crates together
- `core/` – Engine-agnostic domain types such as players, scoring, and boundary definitions
- `adapter/` – Bridges the domain layer into Bevy's ECS and exposes shared resources
- `plugins/` – Reusable Bevy plugins including the design-resolution helper and asset loader
- `scenes/` – Scene implementations (boot flow, title screen) and shared asset handles
- `assets/` – Runtime assets loaded by the game; use `ext-assets/` for externally sourced files

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
