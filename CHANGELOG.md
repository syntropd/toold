# Changelog

All notable changes to toold are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-09-24

### Fixed

- `sd_notify` no longer silently fails on the abstract namespace path
  that stock systemd uses. Implementation now builds the `sockaddr_un`
  via `rustix::net::SocketAddrUnix::new_abstract_name` so the kernel
  receives the correct address for both abstract and filesystem
  targets. Embedded newlines in `STATUS` are stripped to prevent
  early termination of the variable and injection of fake keys.
- `systemd socket activation` now logs `warn!` when an adopted
  FD cannot be converted into a tokio `UnixListener`, instead of
  silently dropping the listener and leaving clients to see
  `connection refused`.
- `varlink GetInfo` reports `env!("CARGO_PKG_VERSION")` instead
  of the hardcoded `0.1.0`.
- `varlink` server now reads NUL-framed JSON envelopes with a
  bounded 1 MiB buffer and replies with `org.varlink.service
  .ProtocolError` if a hostile client never sends a terminator,
  instead of dropping bytes past the first 1024 read and silently
  truncating the call.
- `varlink` listener now selects on a shutdown signal so
  `SIGTERM`/`SIGINT` drain in-flight connections instead of
  dropping them mid-call.
- `toolctl` Varlink error replies now include the daemon's
  parameters payload (e.g. `InvalidParameter` reason, `ToolNotFound`
  name) so operators can see why a call was rejected.

### Added

- Unit tests for `notify_address` validation (filesystem path,
  abstract namespace, empty, interior-NUL rejection) and
  `STATUS` sanitization.
- `release-plz.toml`, CI workflow, and PR title lint for future
  automated releases.

## [0.1.0] - 2026-09-23

### Added

- Initial release: sandboxed agentic action and diagnostic execution
  daemon with rollback journal, Varlink IPC (`io.syntrop.Tool1`),
  and `toolctl` operator CLI.
