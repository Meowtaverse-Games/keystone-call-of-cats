# Web browser runtime handoff

## Resume objective

Continue PR [#72](https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/72)
on branch `feature/web-build-verification`. Fix the browser-only localization
startup panic, then add an actual Chromium runtime smoke test to the Web Build
workflow. Do not merge, deploy, publish Pages, or run the heavy WebAssembly
bundle locally.

The bundle currently builds successfully, but the game does **not** reach a
usable screen in a browser. PR #72 must not be treated as browser-runtime ready
until the acceptance criteria below pass.

## Current repository state

- Base: `main` at `ad30c5c`
- Current head before this handoff commit: `8b1b43c`
- PR: <https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/72>
- Last successful Web Build run:
  <https://github.com/Meowtaverse-Games/keystone-call-of-cats/actions/runs/33300287948>
- Last successful CI run:
  <https://github.com/Meowtaverse-Games/keystone-call-of-cats/actions/runs/33300287966>
- Artifact from that run: `keystone-cc-web`, ID `9728918061`, approximately
  17.3 MB, three-day retention
- The Web Build proves compilation, static file layout, HTTP 200 responses, and
  artifact creation only. It does not currently prove successful app startup.
- User-owned untracked `.DS_Store` files, `.omc/`, `firebase-debug.log`, and
  changes inside the `ext-assets` submodule exist in the worktree. Preserve them
  and never stage them.

## Actual browser test performed

The artifact was served locally and opened with real headless Chromium using
SwiftShader WebGL2. No rebuild was performed on this server.

Artifact directory used:

```text
/tmp/keystone-cc-web-artifact-33300287948
```

HTTP server command:

```sh
python3 -m http.server 8765 \
  --bind 127.0.0.1 \
  --directory /tmp/keystone-cc-web-artifact-33300287948
```

Browser command used on this server:

```sh
/home/ubuntu/.cache/ms-playwright/chromium-1228/chrome-linux/chrome \
  --headless=new \
  --no-sandbox \
  --disable-dev-shm-usage \
  --use-gl=angle \
  --use-angle=swiftshader \
  --enable-unsafe-swiftshader \
  --ignore-gpu-blocklist \
  --enable-logging=stderr \
  --log-level=0 \
  --window-size=1280,720 \
  --virtual-time-budget=20000 \
  --screenshot=/tmp/keystone-web-browser.png \
  --dump-dom \
  http://127.0.0.1:8765/
```

Observed before the failure:

- HTML, generated JavaScript, and Wasm returned HTTP 200.
- A 1280 x 720 canvas was created.
- Bevy initialized a WebGL2 adapter through SwiftShader.
- Runtime image, font, and audio requests returned HTTP 200.
- Stage progress loaded and saved through the Web memory backend.
- The captured screenshot was entirely black.

The local HTTP server was stopped after the test. `/tmp` evidence is ephemeral
and should not be relied on in a later environment. If the artifact has expired,
dispatch `.github/workflows/web-build.yml` on this branch and download the new
`keystone-cc-web` artifact instead of building locally.

## Confirmed runtime blocker

The first material error is:

```text
Reading directories is not supported with the HttpWasmAssetReader
```

It comes from the unconditional folder load at
`src/scenes/boot/systems.rs:53`:

```rust
let locale_folder = asset_server.load_folder("locales");
```

Bevy's browser asset reader cannot enumerate a directory. The empty/incomplete
folder is then passed to `bevy_fluent::LocalizationBuilder::build`. In
`bevy_fluent 0.14.0`, the builder indexes a locale entry that does not exist,
which produces:

```text
panicked at bevy_fluent-0.14.0/src/systems/parameters/mod.rs:50:52:
no entry found for key
Uncaught RuntimeError: unreachable
```

The later `winit` `RefCell already borrowed` panics are secondary fallout from
the first panic and should not be treated as the root cause.

## Recommended implementation direction

Keep the current native `load_folder("locales")` path unchanged. On
`wasm32`, explicitly load the three known `BundleAsset` descriptors instead of
enumerating the directory:

```text
locales/en-US/main.ftl.ron
locales/ja-JP/main.ftl.ron
locales/zh-Hans/main.ftl.ron
```

Each descriptor already references its relative `main.ftl` and `stages.ftl`
files, which the asset loader can fetch directly over HTTP.

A likely design is:

1. Replace or extend `LocaleFolder` with a resource that can represent either
   the native `Handle<LoadedFolder>` or the three Web
   `Handle<bevy_fluent::BundleAsset>` values.
2. On Web, wait until every typed bundle handle is loaded.
3. Construct `Localization` from `Assets<BundleAsset>` in locale fallback order
   and insert it once. `Locale::fallback_chain`, `Localization::new`, and
   `Localization::insert` are public APIs. The bundle locale is available
   through `bevy_fluent::exts::fluent::BundleExt`.
4. Keep the existing native `LocalizationBuilder::build(&LoadedFolder)` path.
5. Factor locale selection/construction into a testable helper where practical.

Do not synthesize a successful localization state when bundles failed to load.
The boot state should wait or report a clear error, not insert an empty
`Localization` that panics later.

Relevant files:

- `src/scenes/boot/systems.rs`
- `src/resources/locale_resources.rs`
- `.github/workflows/web-build.yml`
- `README.md`, if the known limitations change

## Browser smoke test to add to Actions

Extend the existing Web Build job after the static HTTP checks. Use a real
Chromium/Google Chrome available on the standard `ubuntu-latest` runner, serve
`dist/`, allow enough virtual time for the 2.4-second boot timer, and capture
browser stderr plus a screenshot.

The smoke must fail if the browser log contains any of:

```text
panicked at
Uncaught RuntimeError
RuntimeError: unreachable
```

It should also require positive evidence that startup progressed beyond
localization, preferably the existing `Boot timer finished` log and a canvas in
the DOM. Preserve the screenshot as a short-retention artifact on failure, or
include it with the existing three-day Web artifact.

These messages were observed and are not by themselves blockers in headless
Chromium:

- asset watching is unsupported on Web
- AudioContext requires a user gesture
- SwiftShader is software rendering
- GPU preprocessing/OIT/texture binding array capability warnings
- `.meta` and `/favicon.ico` HTTP 404 responses

Do not use a blanket grep for every `ERROR` or `WARN`; it would reject these
expected browser/headless conditions.

## Acceptance criteria

- Web localization does not call `AssetServer::load_folder` on `wasm32`.
- All three locale bundles load by explicit path, with `en-US` fallback intact.
- The app reaches `Boot timer finished` and the stage selection/title UI in
  Chromium without `panicked at`, `Uncaught RuntimeError`, or `unreachable`.
- The browser screenshot is not entirely black and shows the expected first
  usable screen.
- Generated JS, Wasm, and representative audio/image/font/locale/stage assets
  still return HTTP 200.
- `.github/workflows/web-build.yml` performs the Chromium runtime smoke on the
  standard free `ubuntu-latest` runner.
- The Web Build and existing fmt/clippy CI both succeed on the final head.
- Native localization behavior and the Steam Cloud storage-selection fix from
  `8b1b43c` remain unchanged.
- Update PR #72 with the browser evidence and remaining Web limitations.
- Run the normal Terra implementation and Sol independent review flow. Stop
  before merge, deployment, release, or Pages publication.
