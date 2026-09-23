# Windows setup and run

This workflow supports Windows 11 x64 only. It is deliberately split into **SETUP**
(rare machine preparation) and **RUN** (repeatable work in an existing checkout).
macOS support is a future extension; these scripts do not replace the existing macOS
build script.

## SETUP

Transfer `scripts/setup-windows.ps1` as a single file if Git is not installed yet, or
run it from a checkout:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\setup-windows.ps1
```

The script reuses installed Git, MSVC Build Tools, and rustup where available. It
installs missing dependencies through WinGet, asks Windows for elevation when Build
Tools requires it, and does not reboot or relax the machine execution policy. If an
installer requests a reboot, restart Windows and run the same command again.
Open a new PowerShell before cloning so a Git installation's persistent PATH change is
available to that terminal.

`-CheckOnly` does not install or update dependencies. It verifies Rust 1.95 or newer,
the MSVC Rust toolchain, Git, Cargo, and a complete same-version x64 Windows SDK
(`kernel32.lib` and `ucrt.lib`), plus a tiny native Rust compile. `-SkipSmokeTest`
omits only that tiny compile. This is a compiler check, not a Bevy build.

## Clone and RUN

After setup, use the repository URL and branch appropriate to the work:

```powershell
git clone --recurse-submodules https://github.com/Meowtaverse-Games/keystone-call-of-cats.git
cd keystone-call-of-cats
git switch <branch-to-test>
git submodule update --init --recursive
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\run-windows.ps1
```

The runner uses `stable-x86_64-pc-windows-msvc`. It rebuilds the invoking PowerShell
PATH using `CARGO_HOME\bin` first, then machine and user PATH entries, so a newly
installed `rustup.exe` is found without a separate resume command. It gives an
actionable setup/CARGO_HOME error if Rust remains unavailable.

The default is `cargo run --locked --no-default-features`: debug builds retain a
console for diagnostics. `-BuildOnly` changes `run` to `build`; `-Release` adds the
release profile. `-UseDefaultFeatures` opts into `experimental`; `-Features steam`
selects a named feature set. Do not pass both feature options.

Each invocation records transcript output, including the Cargo command and errors,
under `%LOCALAPPDATA%\KeystoneCC`. Include that log, `git rev-parse HEAD`, and the
observed game behavior in Linux feedback.

## Assets

`assets/fonts` and `assets/images` are Git symlinks to the `ext-assets` submodule.
Windows clones without symlink support may check them out as small placeholder files.
The runner replaces only those exact tracked placeholders. It records a fingerprint of
the copied data outside the repository. On future runs it refreshes a copied directory
only when it still matches that recorded copy; if source and destination differ due to
local edits or an unknown previous copy, it stops without deleting anything. Resolve
the asset difference manually before retrying.

## CI, publishing, and local development

These local scripts prepare a developer machine and run the current checkout. They do
not replace either GitHub workflow:

- `.github/workflows/windows_verify.yml` is the trusted-repository Windows verification
  workflow. It builds, runs the Rust test suite, and uploads verification artifacts;
  it does not publish a game or start an A1X machine. Fork pull requests are skipped
  because their private asset submodule cannot receive credentials. The merged workflow
  has passed its 11 Rust tests on Windows.
- `.github/workflows/windows_build.yml` is a manually dispatched distribution workflow
  that includes itch.io publishing. Do not use it as a development or smoke-test command.
- `setup-windows.ps1` and `run-windows.ps1` are for local Windows development only.

The normal game UI has also been started successfully on A1X through the separate,
owner-invoked artifact smoke path. That validation is not a rendered-frame assertion,
and neither local script contacts or controls A1X.
