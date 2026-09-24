# io.syntrop.Tool1: Varlink Interface Specification

The `io.syntrop.Tool1` interface enables authenticated clients to discover, invoke, and revert sandboxed system diagnostic and remediating actions.

Socket Endpoint: `/run/syntrop/io.syntrop.Tool1`

## 1. Interface Definition

```varlink
interface io.syntrop.Tool1

type ToolInfo (
  name: string,
  description: string,
  mode: string,
  timeout_ms: int
)

type ExecutionResult (
  command: string,
  exit_code: int,
  stdout: string,
  stderr: string,
  duration_ms: int
)

type RollbackRecord (
  id: string,
  timestamp_us: int,
  target_path: ?string,
  target_unit: ?string,
  summary: string
)

method ListTools() -> (tools: []ToolInfo)
method ExecuteTool(name: string, args: []string, target_unit: ?string) -> (result: ExecutionResult, rollback_id: ?string)
method Rollback(rollback_id: string) -> (restored: RollbackRecord)
method ListRollbacks(since_seconds: int, limit: int) -> (records: []RollbackRecord)

error ToolNotFound(name: string)
error ExecutionFailed(reason: string)
error PermissionDenied(reason: string)
error Timeout(limit_ms: int)
error InvalidParameter(parameter: string)
```

## 2. Methods

### 2.1 `ListTools`
Retrieves all registered and permitted tools.
- Parameters: none
- Returns:
  - `tools` (`[]ToolInfo`): Array of available tool primitives.

### 2.2 `ExecuteTool`
Executes an allowlisted tool under sandboxed constraints.
- Parameters:
  - `name` (string): Registered tool identifier.
  - `args` (`[]string`): Trailing user-supplied arguments.
  - `target_unit` (?string): Associated systemd unit name.
- Returns:
  - `result` (`ExecutionResult`): Output, exit code, and duration.
  - `rollback_id` (?string): Snapshot identifier if tool is remediating.

### 2.3 `Rollback`
Reverts file changes using a pre-execution snapshot.
- Parameters:
  - `rollback_id` (string): Snapshot identifier.
- Returns:
  - `restored` (`RollbackRecord`): Restored state record.

### 2.4 `ListRollbacks`
Lists historical rollback snapshots.
- Parameters:
  - `since_seconds` (int): History window in seconds.
  - `limit` (int): Maximum records to return.
- Returns:
  - `records` (`[]RollbackRecord`): Historical snapshot array.
