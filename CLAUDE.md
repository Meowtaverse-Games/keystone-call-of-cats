# CLAUDE.md

Guidance for Claude Code (and other coding agents) working in this repo.

## Project Overview
Keystone: Call of Cats — a 2D puzzle game where the player guides cats and stones using a small in-game scripting language. Built on Bevy 0.17 + Avian2D, scripting via Rhai and `keystone-lang`, i18n via `bevy_fluent`. Distributed on itch.io and (optionally) Steam.

## Common Commands
- Build: `cargo build`
- Run (debug): `cargo run`
- Run (release / optimized): `cargo run --release`
- Tests: `cargo test`
- Lint: `cargo clippy`
- macOS .app bundle: `./scripts/build_macos.sh` (set `ENABLE_STEAM=false` for non-Steam builds)
- Deploy to itch.io: `./scripts/deploy_itch.sh` (requires `butler`)
- Sync Japanese translations from Google Sheet → `assets/locales/ja-JP/main.ftl`: `./scripts/export_ftl.sh`

### Cargo features
- `default = ["experimental"]` — gates WIP features. Disable with `--no-default-features`.
- `steam` — opt-in Steamworks integration (`cargo run --features steam`). Without it, the steam plugin and CLI types compile out (`#[cfg(feature = "steam")]`).

## Code Layout
- [src/main.rs](src/main.rs) — App setup, plugin registration, locale detection (`ITCHIO_OFFICIAL_LOCALE` → `LANG` → `en-US`)
- [src/config.rs](src/config.rs) — Constants (currently just the Steam app id)
- [src/plugins/](src/plugins/) — Cross-cutting plugins: `assets`, `design_resolution`, `scripts`, `settings`, `stage`, `steam` (cfg-gated), `tiled`, `visibility`
- [src/scenes/](src/scenes/) — One module per game state plus shared `audio`/`assets`/`options`
  - States live in [src/resources/game_state.rs](src/resources/game_state.rs): `Boot → SelectStage → Stage`, with `Reloading` bouncing back to `SelectStage`
- [src/scenes/stage/](src/scenes/stage/) — Stage gameplay; systems are organized via `StageSystemSet` (Input → Script → Reset → …)
- [src/resources/](src/resources/) — Bevy `Resource`s and shared data types (stage catalog, stone capabilities, settings, launch profile, …)
- [src/systems/](src/systems/) — Engine-level systems (`engine`, `stage`, `steam`)
- [src/util/](src/util/) — Helpers: font loading, localization, script type adapters
- [assets/stages/stage-N.ron](assets/stages/) — Stage definitions in RON; index in `list.ron`
- [assets/locales/](assets/locales/) — Fluent (`.ftl`) translations for `en-US`, `ja-JP`, `zh-Hans`
- [tools/ftl_sheet_exporter/](tools/ftl_sheet_exporter/) — Standalone Rust binary that regenerates `ja-JP/main.ftl` from a Google Sheet (OAuth installed-app flow)
- [scripts/](scripts/) — Build / deploy / FTL-export shell scripts
- [ext-assets/](ext-assets/) — Source files for assets before import/optimization

## Conventions
- Rust **2024 edition** (requires Rust ≥ 1.85).
- Clippy override: `too-many-arguments-threshold = 9` ([clippy.toml](clippy.toml)).
- Asset hot-reload is enabled (`watch_for_changes_override: Some(true)` in `main.rs`); editing files under `assets/` while the game is running picks up changes.
- The primary window is created with `visible: false`. Visibility is toggled later by `VisibilityPlugin` — don't "fix" the hidden-on-launch behavior.
- Source comments are mixed Japanese / English; either is fine. Match the surrounding file.
- Stage logic is organized around `StageSystemSet` ordering — when adding a stage system, place it in the right set rather than relying on `.before/.after` chains.

## Launch Profile (CLI flags)
`LaunchProfile::from_args` ([src/resources/launch_profile.rs](src/resources/launch_profile.rs)) parses CLI args before `App::new()`. Notable `LaunchType`s:
- `ShowChunkGrammarAsciiMap` — prints a stage's chunk-grammar ASCII map and exits
- `SteamAppInfo` — prints Steam app info and exits (requires `steam` feature)
- `render_physics` flag — adds `PhysicsDebugPlugin`

## Watch Out
- [client_secret.json](client_secret.json) at the repo root is the OAuth installed-app credential for `ftl_sheet_exporter`. The cached user token (`tools/ftl_sheet_exporter/.oauth_tokens.json`) must NOT be committed.
- The README's "Notes" section ("Transparent window … typing prints characters …") is stale leftover from an early prototype — ignore it. The current game has nothing to do with text input or window transparency.
- The README describes `tools/` as "sprite sheet exporter" — the actual tool is `ftl_sheet_exporter` (Fluent translation sync). If you update the README, fix this.

## Git Workflow
- Default branch: `main`. Feature branches use `feature/*` or `fix/*`.
- Commit messages and PR titles are written in English; commits are squashed via PR merges (recent history shows `Merge pull request #N from …`).
