# aw-taskwarrior - Linux release (x86_64-unknown-linux-gnu)

Binary: `aw-taskwarrior` (x86-64 ELF, dynamically linked). Checksum in `SHA256SUMS`.

## Provenance

- Source commit: `93ebb6a` on `main` of aw-taskwarrior (built with `cargo build --release` at that commit, release profile with LTO; includes REQ-013 to REQ-016).
- Built with: Rust 1.99.0 (Arch Linux package), glibc 2.44.
- Shared libraries: `libc.so.6`, `libgcc_s.so.1` (both present on standard Linux distributions).
- Tested: unit tests pass (55 watcher tests, 110 across the workspace), a startup smoke test with a closed ActivityWatch port, and REQ-016 logging checked end to end with the real daemon against a scratch aw-server (task command outage and aw-server outage).

## Install

1. Put `task` (TaskWarrior 3.x) on `PATH`.
2. Copy `aw-taskwarrior` to a folder on `PATH`, for example `~/.local/bin`, or to the folder aw-qt searches.
3. Optional settings: `~/.config/activitywatch/aw-taskwarrior/aw-taskwarrior.toml` and `~/.config/activitywatch/aw-client/aw-client.toml`. See the README of the repository for a full example.
4. Connectivity is logged once per change: when the task source or the ActivityWatch server goes down, and again with the outage length when it recovers (REQ-016).
5. Stop the daemon with Ctrl+C or SIGTERM.
