# toolctl: Command-Line Reference

`toolctl` is the companion command-line utility for interacting with the `toold` daemon over Varlink.

## Global Flags

- `-s, --socket <PATH>`: Override Varlink socket path (default: `/run/syntrop/io.syntrop.Tool1`).
- `--json`: Output raw JSON replies instead of formatted tables.
- `-h, --help`: Display help information.
- `-V, --version`: Display version.

## Subcommands

### 1. `list`
List all registered tools, their authorization modes, and timeout thresholds.

```bash
toolctl list [--json]
```

### 2. `run`
Invoke a tool within the toold sandbox.

```bash
toolctl run <TOOL_NAME> [ARGS...] [--unit <TARGET_UNIT>] [--json]
```

Examples:
```bash
# Query active status of nginx
toolctl run unit.status nginx.service

# Read recent journal lines for sshd
toolctl run journal.slice -u sshd.service -n 50
```

### 3. `rollback`
Revert a prior remediating action using its recorded snapshot ID.

```bash
toolctl rollback <ROLLBACK_ID> [--json]
```

Example:
```bash
toolctl rollback rb-1790235247-1
```

### 4. `history`
List recent pre-execution rollback snapshots.

```bash
toolctl history [--since <SECONDS>] [--limit <COUNT>] [--json]
```

Example:
```bash
toolctl history --limit 10
```

### 5. `info`
Introspect daemon vendor metadata and supported Varlink interfaces.

```bash
toolctl info [--json]
```

### 6. `completions`
Generate shell tab completion script for bash, zsh, fish, or powershell.

```bash
toolctl completions <SHELL>
```
