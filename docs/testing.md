# Tests & releases: trust, but do not reboot

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --release
cargo build --locked --release
./target/release/reboot-scheduler --dry-run
```

The unit tests cover startup boundaries, every random offset for representative start times, late polling, clock jumps, rescheduling, entropy rejection and errors, UTC date formatting, argument validation and subprocess status handling. Subprocess tests use `/bin/true`, `/bin/false` and a deliberately missing executable. Integration tests invoke only safe CLI modes. No test calls the reboot command.

## CI and distribution

The GitHub Actions workflow runs formatting, Clippy, tests, and builds a stripped static `x86_64-unknown-linux-musl` executable. It uploads the executable, service unit, MIT license and SHA-256 checksums as an Actions artifact. On a `v0.2.0`-style tag, a separate release job verifies the tag matches Cargo.toml and publishes the same files as a GitHub Release. The workflow runs on pushes, pull requests and manual dispatch.

Download the artifact from Actions, or the assets from Releases after tagging. Verify the checksum before installing:

```sh
sha256sum -c SHA256SUMS
chmod +x reboot-scheduler
./reboot-scheduler --dry-run
```

## Real systemd acceptance

Use a disposable Debian 12/13 VM, not a container sharing a production host. Check the unit and logs, confirm the planned time, and test a real reboot only with explicit authorization and a maintenance window. After the VM boots, check that the new plan targets the next day if the UTC time is within the old window. A successful command status is not evidence that the VM actually rebooted; compare boot IDs and journals.
