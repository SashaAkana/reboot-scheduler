# Architecture: a bedtime story for servers

The program is intentionally one-shot per process. It chooses a target, waits, requests a reboot, then exits. systemd starts it on the next boot. There is no state database, config file, third-party Rust crate or external scheduler.

## The clock is the boss

All calculations use Unix time in whole seconds, independent of the host's local timezone and DST. The fixed window is `[01:00:00, 02:00:00)` UTC.

| Current UTC at startup | Target |
|---|---|
| 00:00:00–00:59:59 | Today's window |
| 01:00:00–23:59:59 | Tomorrow's window |

The second rule is intentional: after a successful reboot, the service typically starts during the same window. Scheduling tomorrow avoids an immediate repeat. It also means a manual service restart during the window skips that day's planned reboot. There is no persistent once-per-calendar-day guarantee across unusual clock changes or manual intervention.

The offset is sampled uniformly from 0–3599 using Linux `/dev/urandom`. Rejection sampling discards the incomplete 16-bit bucket. Concurrent starts no longer share a deterministic timestamp seed. Equal slots remain possible by chance; this is not a distributed coordination mechanism.

## Sleep, check, repeat

```text
start -> choose target -> wait (<= 30s) -> read wall clock
                              ^                |
                              |                +-- before target: wait
                              |                +-- target reached, window open:
                              |                |   recheck -> systemctl -> exit
                              +----------------+-- target window expired:
                                                   pick future target
```

The final sleep is shortened to the time remaining. The decision also compares the clock with the end of the **target's calendar day** window. If the deadline is missed (late wake-up, suspend, time correction), the daemon skips that window and chooses a future target. A backward clock correction delays execution until the original target. Large forward corrections never authorize a reboot based solely on a stale target.

The clock is checked again just before executing `/usr/bin/systemctl --no-block reboot`. This reduces the check-to-call race but does not eliminate it: process scheduling, time corrections, systemd and shutdown latency are outside the daemon's control. The guarantee is about the decision to request a reboot, not the exact instant the hardware restarts. A badly configured host clock can still produce incorrect real-world scheduling.

## What happens when things break?

| Event | Result |
|---|---|
| Entropy or clock error | Error logged; exit 1 |
| Missed window | Reschedule; no late reboot request |
| systemctl returns success | Exit 0; request accepted, not proof that reboot completed |
| systemctl fails or cannot start | Error logged; exit 1 |
| Invalid CLI arguments | Exit 2, no scheduling or reboot |

The supplied unit restarts on failure after 60 seconds. This recomputes the schedule; when a reboot request fails during the window, the next process usually plans tomorrow rather than retrying immediately. A successfully queued request that never results in reboot leaves the one-shot service inactive; operators must investigate and restart it if appropriate. There is deliberately no aggressive retry loop or forced reboot fallback.

The implementation stays in `src/main.rs`: UTC date formatting, entropy, scheduling, pure decision logic, command execution and CLI. Unit tests exercise the decisions without touching the actual reboot command.
