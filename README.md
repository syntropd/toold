# toold

Sandboxed Agentic Action and Diagnostic Execution Daemon for the Syntropd OS Suite.

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](Cargo.toml)

`toold` provides a zero-trust, audited execution sandbox for automated supervisors (`sentry`) and system administrators to run diagnostics and apply guarded remediations without exposing raw root shell access.

---

## Features

- **Strict Tool Allowlisting**: Only verified declarative tool definitions can execute.
- **Pre-Execution Rollback Snapshots**: Automatically captures original configuration state before mutating operations and provides one-click restoration.
- **Timeout and Resource Containment**: Enforces execution timeouts and output buffer truncation to prevent resource exhaustion.
- **Pure Rust Varlink IPC**: Native implementation of `io.syntrop.Tool1` over Unix domain sockets with socket activation.
- **Zero Dynamic C Dependencies**: Directly interfaces with Linux system calls without `libsystemd.so` or `libdbus-1.so`.
- **Minimal Resource Budget**: Target memory footprint under 15 MiB RSS.

---

## Directory Structure

```
toold/
├── Cargo.toml
├── crates/
│   ├── toold-core/          # Core policy engine, sandbox runner, rollback journal
│   ├── toold-daemon/        # Daemon binary: socket activation, Varlink server
│   └── toolctl/             # Admin CLI utility for tool execution and rollback
├── qa/
│   ├── unit/                # 1:1 unit tests for every core and daemon function
│   └── edge/                # Edge cases: timeout enforcement, corrupt journals
├── systemd/                 # toold.service and toold.socket units
├── sysusers.d/              # User and group definitions
├── tmpfiles.d/              # Runtime directories and permissions
├── install/                 # install.sh and uninstall.sh scripts
└── docs/                    # Architecture, Varlink spec, CLI reference
```

---

## Quickstart

### Build and Test

```bash
cargo build --release
cargo test --workspace
```

### Run Daemon Locally

```bash
cargo run --bin toold
```

### Query via toolctl

```bash
# List available tools
cargo run --bin toolctl -- list

# Run a diagnostic tool
cargo run --bin toolctl -- run unit.status nginx.service

# View daemon information
cargo run --bin toolctl -- info
```

---

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
