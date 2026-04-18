# Notidium Install & Release — Design

**Date:** 2026-04-18
**Status:** Draft, awaiting user review
**Scope:** Project A of a two-part effort. Project B (autosave / draft mode) is tracked separately.

## 1. Goals

1. Users install Notidium with a single copy-pasted command (`curl | sh` on macOS/Linux, `iwr | iex` on Windows).
2. Installer runs an interactive onboarding that asks only what it must: vault path and whether to enable auto-start.
3. The server starts automatically at login, so the user never has to remember `notidium serve`.
4. Prebuilt binaries for macOS (arm64/x86_64), Linux (arm64/x86_64, musl), and Windows (x86_64) are produced by GitHub Actions and attached to GitHub Releases on tag push.

## 2. Non-Goals (v1)

- Binary self-update (`notidium self-update`) — revisit in v0.2+.
- Homebrew tap, Scoop bucket, winget, AUR, `.dmg`/`.pkg`.
- macOS code signing and notarization.
- Claude Desktop MCP config auto-wiring — future separate subcommand.
- A real Windows Service (requires admin). A per-user Scheduled Task at logon covers the same need without elevation.
- Autosave / draft mode in the editor — Project B, separate spec.

## 3. Install Layout (Per-User)

- **macOS / Linux**: binary at `~/.notidium/bin/notidium`. Installer appends `~/.notidium/bin` to `PATH` via an idempotent line in the user's shell rc (`~/.zshrc`, `~/.bashrc`, `~/.config/fish/config.fish` — whichever exist).
- **Windows**: binary at `%LOCALAPPDATA%\Notidium\bin\notidium.exe`. Installer prepends that directory to the user `PATH` via `[Environment]::SetEnvironmentVariable('Path', ..., 'User')`.
- Vault directory (user data) stays separate at the user-chosen path (default `~/Notidium`), so reinstalling or deleting the binary never touches notes.
- No sudo / admin required anywhere in the happy path.

Uninstall:
- macOS/Linux: `notidium uninstall-service && rm -rf ~/.notidium/bin` and remove the PATH line.
- Windows: `notidium uninstall-service && rmdir /s %LOCALAPPDATA%\Notidium\bin` and the installer removes its PATH entry.

A future `notidium uninstall` subcommand can bundle these, but is not required for v1.

## 4. Installer Scripts

Two parallel scripts live at the repo root and are served directly from GitHub via `raw.githubusercontent.com`:

- `install.sh` — POSIX sh, for macOS + Linux.
- `install.ps1` — PowerShell, for Windows.

Canonical invocations (documented in README):

```sh
curl -fsSL https://raw.githubusercontent.com/pjankiewicz/notidium/main/install.sh | sh
```

```powershell
iwr -useb https://raw.githubusercontent.com/pjankiewicz/notidium/main/install.ps1 | iex
```

### 4.1 Flow (both scripts)

1. Detect OS + architecture; map to release asset name, e.g. `notidium-aarch64-apple-darwin.tar.gz`.
2. Query `https://api.github.com/repos/pjankiewicz/notidium/releases/latest` for the tag + asset URLs.
3. Download the target tarball/zip and the release's `SHA256SUMS` file into a temp directory.
4. Verify the SHA-256 of the downloaded archive against `SHA256SUMS`. Hard-fail on mismatch.
5. Extract the `notidium` binary to `~/.notidium/bin/` (or Windows equivalent); `chmod +x` on POSIX.
6. **Interactive onboarding**, reading from `/dev/tty` (POSIX) or via `Read-Host` (PowerShell) so prompts still work under `curl | sh` / `iwr | iex`:
   - `Vault path [~/Notidium]:`
   - `Set up auto-start at login? [Y/n]:`
7. If the vault directory is missing or not initialized, run the freshly installed `notidium init <vault>`.
8. If auto-start was chosen, run `notidium install-service --vault <path>` (see §5).
9. Modify the shell rc / user PATH to include the install bin directory. Idempotent — skip if the exact line/entry is already present.
10. Print a short "next steps" block: server URL, log file location, how to uninstall.

### 4.2 Non-Interactive Mode

For CI, provisioning, and scripted installs, the following env vars and flags bypass prompts:

- `NOTIDIUM_VAULT=<path>` — sets vault, skips prompt.
- `NOTIDIUM_INSTALL_SERVICE=1|0` — answer the auto-start prompt.
- `NOTIDIUM_NO_MODIFY_PATH=1` — skip shell rc / PATH modification.
- `-y` / `--yes` flag — accept all defaults for remaining prompts.

### 4.3 Error Handling

- Any failure: print a specific error with remediation hint, remove the temp directory, exit non-zero.
- No partial state: the binary is moved into place only after checksum verification succeeds.
- Network failures surface the HTTP status and URL so users can retry or diagnose proxies.

## 5. Auto-Start Service (New CLI Subcommands)

Three new subcommands in `src/main.rs`, backed by a new `src/service/` module:

- `notidium install-service [--vault PATH] [--port PORT]`
- `notidium uninstall-service`
- `notidium service status` — prints running state and the last N lines of the service log.

### 5.1 Internal Architecture

```
src/service/
├── mod.rs         // trait ServiceManager { install, uninstall, status }
├── macos.rs       // #[cfg(target_os = "macos")]
├── linux.rs       // #[cfg(target_os = "linux")]
└── windows.rs     // #[cfg(target_os = "windows")]
```

Each backend is a small, self-contained unit. Unit generation (plist XML, systemd unit text, Task Scheduler XML) is pure string templating against a typed `ServiceSpec { binary_path, vault_path, port, log_path }` — trivially unit-testable without touching the OS.

### 5.2 Per-Platform Backends

**macOS** — user-level launchd agent:
- File: `~/Library/LaunchAgents/dev.notidium.server.plist`
- Keys: `Label=dev.notidium.server`, `ProgramArguments=[notidium, serve, <vault>, -p, <port>]`, `RunAtLoad=true`, `KeepAlive=true`, `StandardOutPath`/`StandardErrorPath` → `<vault>/.notidium/logs/server.log`.
- Load: `launchctl bootstrap gui/$(id -u) <plist>` on modern macOS, falling back to `launchctl load <plist>` on older versions.

**Linux** — user systemd unit:
- File: `~/.config/systemd/user/notidium.service`
- `Restart=on-failure`; stdout/stderr captured by journald (the `<vault>/.notidium/logs/server.log` path is populated by the server itself; journald is for service-level failures).
- Activate: `systemctl --user daemon-reload && systemctl --user enable --now notidium.service`.
- One-shot prompt: `Enable lingering so Notidium runs without an active login session? [y/N]` → `loginctl enable-linger $USER` if yes.

**Windows** — per-user Task Scheduler task:
- Name: `Notidium`
- Trigger: `At log on of current user`
- Action: `%LOCALAPPDATA%\Notidium\bin\notidium.exe serve <vault> -p <port>`
- Stdout/stderr redirected to `<vault>\.notidium\logs\server.log` (same layout as macOS/Linux, uses the existing `Config::logs_path`).
- Created with `schtasks.exe /create /tn Notidium /tr ... /sc onlogon /rl limited`.
- Task Scheduler does not restart on crash by default; we set `RestartCount=3` and `RestartInterval=PT1M` in the task XML to approximate the `KeepAlive` / `Restart=on-failure` behavior of the other platforms.
- A Windows Service is **not** used — it requires admin and a bespoke service wrapper; the logon task covers the stated goal.

### 5.3 Safety Rules

- Refuse to overwrite an existing plist/unit/task unless `--force` is passed; direct the user to `uninstall-service` first.
- `install-service` calls `notidium init <vault>` if the vault isn't initialized yet, so the service has a valid target on first start.
- `service status` tails the log file so first-run failures are visible without digging.

## 6. GitHub Actions

Two workflows under `.github/workflows/`:

### 6.1 `ci.yml` (PRs + pushes to `main`)

Single `ubuntu-latest` runner:
- `cargo build`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cd frontend && npm ci && npm run lint && npm run type-check && npm run build`

Kept fast; matrix is reserved for release.

### 6.2 `release.yml` (tag push, pattern `v*`)

Matrix of 5 build targets:

| Target | Runner | Notes |
| --- | --- | --- |
| `aarch64-apple-darwin` | `macos-14` | Native Apple Silicon runner |
| `x86_64-apple-darwin` | `macos-13` | Intel macOS |
| `x86_64-unknown-linux-musl` | `ubuntu-latest` | Built via `cross` for static linking |
| `aarch64-unknown-linux-musl` | `ubuntu-latest` | Built via `cross` |
| `x86_64-pc-windows-msvc` | `windows-latest` | `.zip` output |

Per-target job:
1. Checkout.
2. Setup Node, `cd frontend && npm ci && npm run build`. The frontend must be built before `cargo build` because `rust-embed` embeds it into the binary.
3. Setup Rust + `rustup target add <target>` (or install `cross`).
4. `cargo build --release --target <target>`.
5. Package: `notidium-<target>.tar.gz` on POSIX, `notidium-<target>.zip` on Windows. Archive contains the binary, `LICENSE`, and `README.md`.
6. Compute SHA-256, upload archive + sum file as release artifacts.

Aggregation job:
- Download all SHA-256 files, concatenate into a single `SHA256SUMS`, attach to the release.
- Flip the GitHub Release from draft to published so `install.sh`'s `releases/latest` lookup sees it.

### 6.3 Release Trigger

A new `make release` target (or an extension of `make publish`) pushes a `v<version>` tag; CI picks up the tag and produces the release. The existing `cargo publish` flow remains separate for crates.io.

## 7. Error Handling Summary

| Surface | Failure mode | Behavior |
| --- | --- | --- |
| Installer | Network / 404 on asset | Print URL + HTTP status + retry hint, exit non-zero. |
| Installer | Checksum mismatch | Delete temp dir, print expected vs. actual SHA, exit non-zero. |
| Installer | Existing install detected | Warn, ask to overwrite (default No), respect `-y`. |
| `install-service` | Existing plist/unit/task | Refuse unless `--force`; suggest `uninstall-service`. |
| `install-service` | Vault doesn't exist | Run `notidium init <vault>` before installing service. |
| Service runtime | Server crashes | `KeepAlive` (macOS) / `Restart=on-failure` (Linux) / `RestartCount=3` task setting (Windows) restarts it. |
| `service status` | Service not installed | Print "not installed" + next-step hint. |
| `service status` | Service failed to start | Tail last 20 lines of `server.log`. |

## 8. Testing

- **Unit tests** for plist/unit/task generation (`ServiceSpec` → string). Pure functions; no OS interaction.
- **Installer lint**: `shellcheck install.sh` + `Invoke-ScriptAnalyzer install.ps1` in CI.
- **Installer unit tests** (bats for sh, Pester for PowerShell) covering OS/arch detection and idempotent PATH patching.
- **CI smoke test** per release runner: after packaging, extract the artifact, run `notidium --version`, run `notidium install-service --vault <tmp>` with `NOTIDIUM_SKIP_DAEMON_START=1` (a test-only env var the service module honors to skip `launchctl`/`systemctl`/`schtasks` activation), then `notidium uninstall-service`.
- **Manual verification checklist** (added to README or a `docs/testing/release-checklist.md`): fresh macOS, Ubuntu, and Windows VMs — run the install one-liner, reboot, confirm server answers on port 3939.

## 9. Files Touched / Added

**New:**
- `install.sh` (repo root)
- `install.ps1` (repo root)
- `src/service/mod.rs`, `src/service/macos.rs`, `src/service/linux.rs`, `src/service/windows.rs`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `docs/testing/release-checklist.md` (optional, manual checklist)

**Modified:**
- `src/main.rs` — add `InstallService`, `UninstallService`, `ServiceStatus` subcommands.
- `Cargo.toml` — minor deps if needed (likely `plist` crate for macOS, none required for others).
- `Makefile` — new `release` target that tags and pushes.
- `README.md` — one-liner install instructions, auto-start docs.

## 10. Rollout Order

1. CI workflow (`ci.yml`) — catches regressions early.
2. `release.yml` + first tagged release produces downloadable artifacts. Validate manually by downloading and running.
3. `install-service` subcommand + `src/service/` module with unit tests.
4. `install.sh`, then `install.ps1`. Test against the existing release artifacts.
5. Update README, cut a release with the new install story documented.

Each step is independently shippable and verifiable.
