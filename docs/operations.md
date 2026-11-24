# Operations: please do not test this on production

## Requirements

Linux, systemd as the host service manager, `/usr/bin/systemctl`, readable `/dev/urandom`, and root privileges for service installation and normal reboot requests. Intended for Debian 12/13 and comparable Linux systems; live compatibility must be verified on the target host. No Windows runtime support. Rust 1.85+ and Cargo are needed only to build from source.

## Build

```sh
cargo build --release --locked
file target/release/reboot-scheduler
ldd target/release/reboot-scheduler
./target/release/reboot-scheduler --version
./target/release/reboot-scheduler --dry-run
```

The native GNU build generally links dynamically against glibc. Build on the oldest deployment distribution or use the static musl build provided by CI. Release settings: size optimization (`z`), LTO, one codegen unit, stripped symbols, abort on panic. Cargo.lock is tracked; the CI toolchain follows stable, so byte-for-byte reproducibility is not promised.

## Install (deliberately not started yet)

Run these commands only on a host you administer, initially a disposable VM:

```sh
sudo install -Dm755 target/release/reboot-scheduler /opt/reboot-scheduler/reboot-scheduler
sudo install -Dm644 reboot-scheduler.service /etc/systemd/system/reboot-scheduler.service
sudo systemd-analyze verify /etc/systemd/system/reboot-scheduler.service
sudo systemctl daemon-reload
/opt/reboot-scheduler/reboot-scheduler --dry-run
```

When you have explicitly approved automatic daily reboots on that host:

```sh
sudo systemctl enable --now reboot-scheduler.service
sudo systemctl status reboot-scheduler.service
sudo journalctl -u reboot-scheduler.service -b --no-pager
```

The preview independently samples a slot; the real service samples its own slot. Read the service journal for its actual plan.

## CLI and configuration

| Command | Effect |
|---|---|
| `reboot-scheduler --dry-run` | Print a sample target and exit without invoking systemctl |
| `reboot-scheduler --help` | Usage |
| `reboot-scheduler --version` | Package version |
| `reboot-scheduler` | Wait and request a real reboot |

The UTC window, polling limit and absolute command path are compile-time constants. No environment-variable overrides, config files or runtime custom commands are provided.

## Inspect, stop, update

```sh
sudo journalctl -u reboot-scheduler.service -f
sudo systemctl stop reboot-scheduler.service
sudo systemctl disable reboot-scheduler.service
```

Stopping the daemon does not cancel a reboot already queued with systemd. To update, stop the service first, replace the binary and unit using the install commands above, run verify and daemon-reload, then start or enable it only after confirming the maintenance policy. Restarting during or after the window plans for tomorrow.

## Remove

```sh
sudo systemctl disable --now reboot-scheduler.service
sudo rm -f /etc/systemd/system/reboot-scheduler.service
sudo rm -f /opt/reboot-scheduler/reboot-scheduler
sudo rmdir /opt/reboot-scheduler
sudo systemctl daemon-reload
```

## Privileges

The service runs as root with `CAP_SYS_BOOT` as its capability bounding set and ambient capability, `NoNewPrivileges=true`, `ProtectSystem=strict`, `ProtectHome=true` and `PrivateTmp=true`. It still has the ability to reboot the host. These restrictions are not a security boundary against all privileged actions. Verify the unit and inspect hardening with `systemd-analyze security reboot-scheduler.service` on the actual target. No live reboot is needed to run `--dry-run`.
