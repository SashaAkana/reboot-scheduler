# Changelog

## 0.2.0

- Fix stale-target reboot decisions after the maintenance window expires.
- Shorten the final polling sleep and recheck the wall clock before requesting reboot.
- Use Linux OS entropy with unbiased sampling instead of a seconds-seeded PRNG.
- Fail explicitly on invalid clock or entropy errors.
- Add safe `--dry-run`, `--help` and `--version` modes.
- Use an absolute systemctl path; retain one-shot service semantics.
- Add regression tests, CI, release artifacts and separate technical documentation.
- Publish under the MIT License; refresh the README title and license badge.

## 0.1.1

- Original AWG_Harness reboot-module prototype with a fixed UTC window and zero external dependencies.
