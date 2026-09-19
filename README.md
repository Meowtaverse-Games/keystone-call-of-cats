# Keystone: Call of Cats
Keystone: Call of Cats invites you to guide curious cats and stones toward a shared goal while discovering how to program along the way.

## Repository Layout
- `src/main.rs` – Binary entrypoint that configures Bevy and registers the game layers
- `src/config.rs` – Game configuration constants and settings
- `src/plugins/` – Modular game features and engine integrations
- `src/resources/` – Shared game state and data structures
- `src/scenes/` – High-level game states (`Boot`, `SelectStage`, `Stage`) and screen definitions
- `src/systems/` – Game logic systems (input handling, movement, collision, etc.)
- `src/util/` – Helper functions and common utilities
- `assets/` – Runtime assets (images, audio, fonts, locales, stage data)
- `scripts/` – Build, packaging, and deployment scripts
- `tools/` – Internal development tools (e.g., `ftl_sheet_exporter` for syncing translations from Google Sheets)
- `ext-assets/` – Source files gathered from external tools/artists before import or optimization

## How To Run

- Prerequisites: Rust toolchain (stable, ≥ 1.95 for the current dependency set) via `rustup`.
- Run (debug): `cargo run`
- Run (optimized): `cargo run --release`

### Cargo features
- `experimental` (default) – gates work-in-progress features. Disable with `--no-default-features`.
- `steam` – opt-in Steamworks integration: `cargo run --features steam`.

## Windows development (Windows 11 x64)

Windows setup and everyday game execution are separate. The setup script installs or
checks Git, MSVC C++ Build Tools (including the Windows SDK), and the Rust MSVC
toolchain. Run it from an elevated-capable PowerShell on the Windows machine; it may
show UAC and ask for a reboot, but never reboots or changes the persistent execution
policy itself.

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\setup-windows.ps1
```

Clone with submodules after setup. Private submodule access may require normal GitHub
authentication. Select the branch you want to test; no branch or commit is pinned by
these instructions.

```powershell
git clone --recurse-submodules https://github.com/Meowtaverse-Games/keystone-call-of-cats.git
cd keystone-call-of-cats
git switch <branch-to-test>
git submodule update --init --recursive
```

For every run, call the runner from any directory. It locates its own checkout, repairs
the current process PATH from `CARGO_HOME` (or `%USERPROFILE%\.cargo`), and writes
transcript output under `%LOCALAPPDATA%\KeystoneCC`. Debug run without experimental
features is the default, matching the non-release CI feature choice.

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\run-windows.ps1
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\run-windows.ps1 -BuildOnly -Release
```

Use `-UseDefaultFeatures` for the normal Cargo default, or `-Features steam` for a
specific feature set. The runner never clones, resets, switches branches, or pulls.
It prepares Windows symlink-placeholder assets from `ext-assets` once. Later source
asset changes are refreshed only when the previous copied destination is unchanged;
local asset edits stop the run with an instruction instead of being overwritten.

`setup-windows.ps1 -CheckOnly` performs checks without package installation. Add
`-SkipSmokeTest` when only dependency detection is wanted. macOS is not implemented
by these Windows scripts; keep using `scripts/build_macos.sh` for its existing flow.
See [design/windows-setup.md](design/windows-setup.md) for the fuller Windows notes.

### Localization
The game ships with `en-US`, `ja-JP`, and `zh-Hans` locales under `assets/locales/`. The initial locale is picked from `ITCHIO_OFFICIAL_LOCALE`, then `LANG`, falling back to `en-US`. The user's choice is persisted in the game settings file.

## Code License

- Scope: All source code outside of the `assets/` directory (`src/`, `scripts/`, build files, documentation, etc.)
- License: GNU General Public License v3.0 — see `LICENSE`

## Assets License

- Scope: All images, audio, fonts, and other media files inside the `assets/` directory
- License: Each subfolder of `assets/` contains its own `LICENSE` file, which governs that folder’s contents
