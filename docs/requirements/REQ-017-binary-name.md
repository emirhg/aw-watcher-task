# REQ-017: Name the executable aw-watcher-task

- **Status:** Documented, not implemented
- **Added:** 2026-11-20
- **Owner:** maintainer

## Rationale

The owner wants the executable to be called `aw-watcher-task`, so the file that runs as the watcher matches the name the owner uses for it. Only the executable file name changes. The package and crate names, the config folder, the ActivityWatch client name and the bucket id stay as they are, so existing configuration, history and the single-instance lock keep working.

## Design

### What changes

- The Cargo binary target in `crates/aw-taskwarrior/Cargo.toml` is named `aw-watcher-task`. Its source path stays `src/main.rs`.
- Linux: `target/release/aw-watcher-task` (and `target/debug/aw-watcher-task`).
- Windows: `target/windows/x86_64-pc-windows-gnu/release/aw-watcher-task.exe`. `scripts/build-windows.sh` builds and names this file.
- README, CLAUDE.md, REQ-011 and `docs/modules.md` name the new file.

### What does not change

| Item | Stays as | Why |
|------|----------|-----|
| Package and crate names | `aw-taskwarrior`, `aw-taskwarrior-core`, `aw-taskwarrior-provider-*` | `cargo build -p aw-taskwarrior`, features and `Cargo.lock` keep working. Renaming crates is a larger change. |
| Config folder and file | `~/.config/activitywatch/aw-taskwarrior/aw-taskwarrior.toml` (Windows: `%LOCALAPPDATA%\activitywatch\aw-taskwarrior\`) | Existing config keeps working. |
| Config section | `[aw-taskwarrior]` | Same as above. |
| ActivityWatch client name | `aw-taskwarrior` | Also names the single-instance lock file. |
| Bucket id | `aw-watcher-taskwarrior_<hostname>` | Existing events stay in one bucket. |
| Startup log label | `Starting aw-taskwarrior` | Log text only. |

### Consequences

- **Old and new processes at the same time.** On Windows the single-instance guard does not stop a second daemon: `aw-client-rust` removes the lock file before creating it. If the old `aw-taskwarrior.exe` is still running when `aw-watcher-task.exe` starts, both post heartbeats to the same bucket, with possibly different pulsetimes (see REQ-016 notes). Stop the old process before starting the new one.
- **Autostart and launchers.** Anything that starts the daemon by name must be updated: aw-qt autostart entries, shortcuts, services, scripts.
- **Release branches.** `release/linux-gnu/x86_64-pc` and `release/windows-gnu/x86_64-pc` still ship files named `aw-taskwarrior` until they are rebuilt.
- **Installed copies.** A copy already installed (for example `~/.local/bin/aw-taskwarrior`) keeps running its old code until it is replaced.

## Acceptance criteria

- [ ] `cargo build -p aw-taskwarrior` produces `target/debug/aw-watcher-task`.
- [ ] `cargo build --release -p aw-taskwarrior` produces `target/release/aw-watcher-task`.
- [ ] `scripts/build-windows.sh` produces `aw-watcher-task.exe` in `target/windows/x86_64-pc-windows-gnu/release/`.
- [ ] The daemon with a scratch config folder starts and finds the config under `aw-taskwarrior/`, so existing settings still apply.
- [ ] README, CLAUDE.md, REQ-011 and `docs/modules.md` name `aw-watcher-task` for the executable and keep the package name `aw-taskwarrior`.

## Code references

- Implemented in: (empty until implemented)
