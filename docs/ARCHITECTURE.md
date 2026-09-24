# toold: Architecture and System Design

`toold` is the Sandboxed Action and Diagnostic Execution Daemon for the Syntropd operating system suite. It provides a secure execution environment for automated supervisor daemons like `sentry` and human administrators.

## 1. Problem Statement

Autonomous incident remediation requires system interactions:
1. Reading diagnostic journals (`journalctl -u nginx`).
2. Querying kernel telemetry (`cgroups v2` PSI counters, `/proc/net`).
3. Verifying service configurations (`systemd-analyze verify`).
4. Applying configuration drop-ins and restarting units.

Executing arbitrary shell scripts (`bash -c`) as root introduces high risk of accidental damage, infinite loops, resource starvation, or security compromise. `toold` acts as a zero-trust mediator and execution boundary.

## 2. Core Architecture

```
                  +-----------------------------------+
                  |               toold               |
                  |                                   |
  Varlink IPC --> | [ToolRegistry] -> Policy Check    |
                  | [RollbackJournal] (Pre-Snapshot)  |
                  | [SandboxRunner] -> Child Process  |
                  +-----------------+-----------------+
                                    |
                                    v
                     /run/syntrop/io.syntrop.Tool1
                                    |
          +-------------------------+-------------------------+
          |                                                   |
          v                                                   v
       sentry                                              toolctl
 (Autonomous Supervisor)                             (Operator CLI Tool)
```

## 3. Subsystem Overview

### 3.1 Declarative Policy Engine (`toold-core::policy`)
- **Permission Modes**:
  - `ReadOnly`: Read-only diagnostics. Cannot modify system state.
  - `RemediateWithRollback`: Mutating operations. Requires automatic pre-execution snapshots.
  - `Guarded`: Actions requiring explicit operator elevation.
- **Tool Definitions**: Fixed executable binary paths, immutable argument prefixes, strict execution timeouts, and permitted path allowlists.

### 3.2 Rollback Journaling (`toold-core::journal`)
- Prior to executing a mutating tool, `toold` captures a complete pre-modification snapshot of the target configuration file.
- Snapshots are committed atomically to `/var/lib/toold/rollback.jsonl`.
- If an applied remediation fails health checks, invoking `Rollback` restores the file to its original content or unlinks newly created drop-ins.

### 3.3 Sandboxed Runner (`toold-core::sandbox`)
- Child processes run with sanitized environments (`PATH=/usr/bin:/bin`, `LANG=C.UTF-8`).
- Strict timeout enforcement via asynchronous timers (`tokio::time::timeout`). If a process hangs, SIGKILL is sent immediately.
- Buffer-limited stdout/stderr collection prevents denial-of-service memory exhaustion.

### 3.4 Varlink IPC Contract (`toold-daemon::varlink`)
- Standard NUL-terminated JSON messaging over `/run/syntrop/io.syntrop.Tool1`.
- Implements `org.varlink.service` introspection.
- Implements `io.syntrop.Tool1` (`ListTools`, `ExecuteTool`, `Rollback`, `ListRollbacks`).

## 4. Resource Budgets and Security

- **Memory Footprint**: Target RSS < 15 MiB.
- **Zero Dynamic C Dependencies**: Compiles directly against standard Linux syscalls without `libsystemd.so` or `libdbus-1.so`.
- **Systemd Sandboxing**: `ProtectSystem=strict`, `MemoryDenyWriteExecute=yes`, `SystemCallFilter=@system-service`.
