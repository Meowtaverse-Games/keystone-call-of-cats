# Web browser runtime handoff (2026-08-31)

Runtime implementation was verified at `697d0ef` on `feature/web-build-verification`, PR [#72](https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/72). This document is committed later and is documentation only.

## Verified browser readiness

- [Web Build #33341697984](https://github.com/Meowtaverse-Games/keystone-call-of-cats/actions/runs/33341697984) passed on standard `ubuntu-latest` in 19m18s; [CI #33341697978](https://github.com/Meowtaverse-Games/keystone-call-of-cats/actions/runs/33341697978) passed fmt and clippy.
- Artifacts expire 2026-09-02 23:42 UTC: `keystone-cc-web-browser-smoke` (ID 9740980471, 53,357 bytes), `keystone-cc-web` (ID 9740981038, 17,302,634 bytes).
- Chromium logged `Assets loaded` → `Boot timer finished` → `Playing BGM` → `Stage selection UI spawned 23 entries`, with no panic/unreachable. The 1280×720, 50,873-byte screenshot was visually checked: KEYSTONE + CALL OF CATS, OPTIONS/EXIT, Stage 1–3, PLAY, pager; the stdlib UI contrast check passed.
- The three explicit locale `.ron`/`.ftl` files and representative audio/image/font/stage assets returned HTTP 200.

Native keeps `load_folder`, `LocalizationBuilder`, and its 2400ms delay. Web explicitly loads three bundles and has a 0ms splash. Steam Cloud selection remains unchanged. Startup and stage UI are ready; full Web gameplay parity is not proven.

## Manual Chrome blockers

Title/stage select works, but a user reported no audio and F3 Run does not start. Do not claim either is fixed.

### P0 audio unlock

Defaults are master .8/music .7/SFX .9, assets are HTTP 200, and `Playing BGM` is logged, yet Chrome reports AudioContext autoplay rejection. Strongest current hypothesis: cpal 0.15.3 webaudio calls `ctx.resume()` once during startup without a useful gesture-time retry; game click paths only spawn `AudioPlayer` entities. User console evidence is not yet captured, so this is likely, not proven. Add a web-only first pointer/key bridge that resumes the actual cpal context, or redesign output initialization/recreation. Review `played_bgm`: marking it true before real output can block retry. Add click SFX/BGM browser smoke.

Acceptance: after first gesture BGM and click SFX are audible; no console panic; native audio regression-free.

### P1 F3 Web UX

README's script limitation is real: UI shows F3 enabled, but Keystone/Rhai uses `std::thread::spawn` plus sync channel, unsupported in wasm. Short term, hide/disable F3 on wasm with an explicit reason. Full solution: P2 cooperative frame-by-frame nonblocking runner, applied to native too for deterministic test/cancel/replay. Web Worker is an alternative but adds COOP/COEP and distribution complexity.

Acceptance: clear unsupported F3 UI at minimum; full implementation moves scripts in Web without panic and retains native checks.

## Proposed, not implemented

Priority: P0 capability resource for UI/feature gates and audio lifecycle (focus/device loss/resume/retry); P1 OS-thread-independent script runner and explicit boot-readiness state; P2 investigate duplicate StageSelect spawn after initial Locale reload, manifest/explicit-asset determinism, separate browser interaction from native startup smoke, PNG checker tests/name; P3 cache/preinstall Trunk (currently about nine minutes).

## Next session and boundaries

1. Reproduce P0 with real-user console evidence, then audio unlock.
2. Implement P1 wasm F3 gate.
3. Deliver P2 cooperative runner incrementally.

Avoid heavy local wasm bundles; Actions uses standard hosted runners. Preview URLs are temporary and stop at session end. No merge, deploy, release, or Pages publication occurred. Preserve and never stage/revert user-owned `.DS_Store`, `.omc/`, `firebase-debug.log`, or `ext-assets` dirt.
