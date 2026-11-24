# Reboot Scheduler

> Because uptime isn't a personality trait.

![Rust](https://img.shields.io/badge/Rust-2021-de803a?style=flat-square&logo=rust&logoColor=white) ![Linux](https://img.shields.io/badge/Linux-systemd-30363d?style=flat-square&logo=linux&logoColor=white) ![Version](https://img.shields.io/badge/version-0.2.0-f0a04b?style=flat-square) ![Dependencies](https://img.shields.io/badge/dependencies-0-3fb950?style=flat-square) [![License: MIT](https://img.shields.io/badge/license-MIT-30363d?style=flat-square)](LICENSE)

A tiny Rust daemon that gives a Linux host a randomly scheduled reboot between **01:00 and 02:00 UTC**. No cron maze, no configuration framework, no dependency collection. Just a clock, a little entropy and systemd.

```text
$ reboot-scheduler --dry-run
Next reboot: 2026-11-25 01:37:42 UTC (dry run; no reboot requested)
```

*Example output. Your server gets its own bedtime.*

### What it does

- Picks a random second in the daily UTC maintenance window.
- Skips a missed window instead of rebooting the host late.
- Avoids scheduling another reboot in the same window after a restart.
- Asks systemd for a reboot and lets it handle the actual shutdown.

### Try it (without turning anything off)

On Linux, with Rust 1.85 or newer and Cargo:

```sh
cargo build --release --locked
./target/release/reboot-scheduler --dry-run
```

**Heads up:** running without arguments waits for a real reboot. Enabling the supplied service authorizes automatic host reboots. Use a disposable VM for live tests, not a production host.

### The boring bits, with some personality

- [How the clock works](docs/architecture.md) — scheduling, entropy and failure semantics.
- [Build & install](docs/operations.md) — systemd, logs, updates and removal.
- [Tests & releases](docs/testing.md) — how to check it without rebooting anything.
- [Changelog](CHANGELOG.md)

Originally a prototype for the reboot module of AWG_Harness; now a standalone personal project.

### License

Licensed under the [MIT License](LICENSE).

---

*Have you tried turning it off and on again? I automated that.*
