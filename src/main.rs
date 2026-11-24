//! A small, one-shot reboot scheduler for Linux/systemd.
//! Even servers need a nap. Prefer a scheduled one.

use std::fs::File;
use std::io::{self, Read};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DAY: u64 = 86_400;
const WINDOW_START_SEC: u64 = 3_600;
const WINDOW_END_SEC: u64 = 7_200;
const WINDOW_LEN: u64 = WINDOW_END_SEC - WINDOW_START_SEC;
const POLL_INTERVAL: u64 = 30;
const SYSTEMCTL: &str = "/usr/bin/systemctl";
const REBOOT_ARGS: &[&str] = &["--no-block", "reboot"];

fn fmt_time(ts: u64) -> String {
    let days_total = ts / DAY;
    let time_of_day = ts % DAY;

    let hh = time_of_day / 3600;
    let mm = (time_of_day % 3600) / 60;
    let ss = time_of_day % 60;

    // Gregorian civil date from days since the Unix epoch.
    let z = days_total as i64 + 719468;
    let era: i64 = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02} UTC")
}

fn unix_now() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| format!("clock is before the Unix epoch: {e}"))
}

fn log(msg: &str) {
    let stamp = unix_now()
        .map(fmt_time)
        .unwrap_or_else(|_| "invalid clock".into());
    eprintln!("[{stamp}] {msg}");
}

// Rejection sampling: 65,536 is not divisible by 3,600.
// Reject the incomplete bucket so each second is equally likely.
fn random_offset_from(reader: &mut impl Read) -> io::Result<u64> {
    let limit = (u64::from(u16::MAX) + 1) / WINDOW_LEN * WINDOW_LEN;
    loop {
        let mut bytes = [0; 2];
        reader.read_exact(&mut bytes)?;
        let sample = u64::from(u16::from_le_bytes(bytes));
        if sample < limit {
            return Ok(sample % WINDOW_LEN);
        }
    }
}

fn next_reboot_ts_from(now: u64, offset: u64) -> u64 {
    let midnight = now - now % DAY;
    let base = if now % DAY < WINDOW_START_SEC {
        midnight
    } else {
        midnight + DAY
    };
    base + WINDOW_START_SEC + offset % WINDOW_LEN
}

fn schedule(now: u64) -> Result<u64, String> {
    let source = File::open("/dev/urandom");
    let mut entropy = source.map_err(|e| format!("cannot open /dev/urandom: {e}"))?;
    let sample = random_offset_from(&mut entropy);
    let offset = sample.map_err(|e| format!("cannot read /dev/urandom: {e}"))?;
    Ok(next_reboot_ts_from(now, offset))
}

#[derive(Debug, PartialEq, Eq)]
enum Decision {
    Wait(u64),
    Reboot,
    Reschedule,
}

// A missed window is never permission to reboot late. The target's date
// matters as much as the time of day, including after clock jumps.
fn decision(now: u64, target: u64) -> Decision {
    let end = target - target % DAY + WINDOW_END_SEC;
    if now >= end {
        Decision::Reschedule
    } else if now >= target {
        Decision::Reboot
    } else {
        Decision::Wait((target - now).min(POLL_INTERVAL))
    }
}

fn do_reboot_with(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| format!("cannot start `{program}`: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`{program}` failed: {status}"))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Run,
    DryRun,
    Help,
    Version,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Mode, String> {
    let mode = match args.next().as_deref() {
        None => Mode::Run,
        Some("--dry-run") => Mode::DryRun,
        Some("--help" | "-h") => Mode::Help,
        Some("--version" | "-V") => Mode::Version,
        Some(arg) => return Err(format!("unknown argument: {arg}")),
    };
    if args.next().is_some() {
        return Err("expected at most one argument".into());
    }
    Ok(mode)
}

fn announce(target: u64) {
    log(&format!("Next reboot: {}", fmt_time(target)));
}

fn run() -> Result<(), String> {
    log("Scheduler started. Even servers need a nap.");
    let mut target = schedule(unix_now()?)?;
    announce(target);
    let mut last_report = std::time::Instant::now();
    loop {
        let now = unix_now()?;
        match decision(now, target) {
            Decision::Wait(seconds) => {
                if last_report.elapsed() >= Duration::from_secs(3_600) {
                    log(&format!("Next reboot in {} min", (target - now) / 60));
                    last_report = std::time::Instant::now();
                }
                thread::sleep(Duration::from_secs(seconds));
            }
            Decision::Reschedule => {
                log("Missed reboot window; skipping it, not rebooting late.");
                target = schedule(now)?;
                announce(target);
                last_report = std::time::Instant::now();
            }
            Decision::Reboot => {
                log("Target reached. Checking the clock before requesting reboot.");
                // Recheck after logging in case the clock or scheduler moved.
                if decision(unix_now()?, target) != Decision::Reboot {
                    continue;
                }
                do_reboot_with(SYSTEMCTL, REBOOT_ARGS)?;
                log("Reboot request accepted by systemctl (not proof of a completed reboot).");
                return Ok(());
            }
        }
    }
}

fn main() -> ExitCode {
    let mode = match parse_args(std::env::args().skip(1)) {
        Ok(mode) => mode,
        Err(e) => {
            eprintln!("{e}; see --help");
            return ExitCode::from(2);
        }
    };
    let result = match mode {
        Mode::Help => {
            println!("reboot-scheduler [--dry-run | --help | --version]\n");
            println!("No args: wait for a random 01:00–02:00 UTC slot and request a real reboot.");
            println!("--dry-run: print the planned time and exit; never invoke systemctl.");
            Ok(())
        }
        Mode::Version => {
            println!("reboot-scheduler {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Mode::DryRun => unix_now().and_then(schedule).map(|target| {
            let time = fmt_time(target);
            println!("Next reboot: {time} (dry run; no reboot requested)");
        }),
        Mode::Run => run(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log(&format!("ERROR: {e}"));
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn before_window_uses_today() {
        assert_eq!(next_reboot_ts_from(3599, 123), 3723);
    }

    #[test]
    fn at_start_and_later_use_tomorrow() {
        for now in [3600, 3601, 7199, 7200, 86399] {
            assert_eq!(next_reboot_ts_from(now, 0), DAY + 3600);
        }
    }

    #[test]
    fn all_offsets_stay_in_window() {
        for now in [0, 3599, 3600, 7199, 7200, 86399, 1_700_000_000] {
            for offset in 0..WINDOW_LEN {
                let target = next_reboot_ts_from(now, offset);
                assert!(target > now);
                assert!((WINDOW_START_SEC..WINDOW_END_SEC).contains(&(target % DAY)));
            }
        }
    }

    #[test]
    fn waits_at_most_30_seconds_and_shortens_final_sleep() {
        assert_eq!(decision(3500, 3600), Decision::Wait(30));
        assert_eq!(decision(3599, 3600), Decision::Wait(1));
    }

    #[test]
    fn target_and_last_second_are_allowed() {
        assert_eq!(decision(3600, 3600), Decision::Reboot);
        assert_eq!(decision(7199, 7199), Decision::Reboot);
    }

    #[test]
    fn late_poll_and_end_boundary_never_reboot() {
        assert_eq!(decision(7200, 7195), Decision::Reschedule);
        assert_eq!(decision(7220, 7195), Decision::Reschedule);
    }

    #[test]
    fn forward_clock_jump_never_reboots_in_old_window() {
        for now in [43200, DAY + 3600, 2 * DAY + 4000] {
            assert_eq!(decision(now, 4000), Decision::Reschedule);
        }
    }

    #[test]
    fn backward_clock_jump_waits_for_original_target() {
        assert_eq!(decision(DAY + 3599, DAY + 3600), Decision::Wait(1));
        assert_eq!(decision(4000, DAY + 3600), Decision::Wait(30));
    }

    #[test]
    fn rescheduling_after_missed_window_uses_future_slot() {
        let now = 7200;
        let target = next_reboot_ts_from(now, 0);
        assert_eq!(target, DAY + 3600);
        assert_eq!(decision(now, target), Decision::Wait(30));
    }

    #[test]
    fn entropy_rejects_incomplete_bucket() {
        let bytes = [255, 255, 42, 0];
        let mut reader = &bytes[..];
        assert_eq!(random_offset_from(&mut reader).unwrap(), 42);
        assert!(reader.is_empty());
    }

    #[test]
    fn entropy_failure_is_not_silently_ignored() {
        let mut empty: &[u8] = &[];
        assert!(random_offset_from(&mut empty).is_err());
    }

    #[test]
    fn utc_format_handles_epoch_leap_day_and_year_rollover() {
        assert_eq!(fmt_time(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(fmt_time(1_709_164_800), "2024-02-29 00:00:00 UTC");
        assert_eq!(fmt_time(1_735_689_600), "2025-01-01 00:00:00 UTC");
    }

    #[test]
    fn cli_never_treats_unknown_arguments_as_run() {
        let parse = |args: &[&str]| parse_args(args.iter().map(|s| (*s).to_owned()));
        assert_eq!(parse(&[]).unwrap(), Mode::Run);
        assert_eq!(parse(&["--dry-run"]).unwrap(), Mode::DryRun);
        assert_eq!(parse(&["--help"]).unwrap(), Mode::Help);
        assert_eq!(parse(&["--version"]).unwrap(), Mode::Version);
        assert!(parse(&["--dryrun"]).is_err());
        assert!(parse(&["--dry-run", "extra"]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn command_success() {
        assert!(do_reboot_with("/bin/true", &[]).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn command_failure() {
        assert!(do_reboot_with("/bin/false", &[]).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn command_missing() {
        assert!(do_reboot_with("/definitely/not/exist", &[]).is_err());
    }
}
