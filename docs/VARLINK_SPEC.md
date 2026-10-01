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

---

# io.syntrop.Actuator1: Varlink Interface Specification

The `io.syntrop.Actuator1` interface enables authorized callers to emit virtual HID keyboard and mouse events via the Linux kernel `/dev/uinput` subsystem.

Socket Endpoint: `/run/syntrop/io.syntrop.Actuator1` (symlink to `/run/syntrop/io.syntrop.Tool1`)

## 1. Interface Definition

```varlink
interface io.syntrop.Actuator1

method SendKey(key_code: int, down: bool) -> ()
method TypeText(text: string) -> ()
method MoveMouse(dx: int, dy: int) -> ()
method ClickMouse(button: int) -> ()

error ActuatorUnavailable(reason: string)
error DeviceError(reason: string)
error InvalidParameter(parameter: string)
```

## 2. Methods

### 2.1 `SendKey`
Emits a low-level evdev key press or release event.
- Parameters:
  - `key_code` (int): Linux evdev key code (e.g., 30 for KEY_A, 28 for KEY_ENTER).
  - `down` (bool): True for key press, false for key release.
- Returns: empty

### 2.2 `TypeText`
Emits an ASCII/UTF-8 string with automatic shift modifiers and press/release sequences.
- Parameters:
  - `text` (string): Text string to type.
- Returns: empty

### 2.3 `MoveMouse`
Moves the mouse pointer by relative delta offsets in pixels.
- Parameters:
  - `dx` (int): Horizontal pixel offset.
  - `dy` (int): Vertical pixel offset.
- Returns: empty

### 2.4 `ClickMouse`
Emits a mouse button press followed immediately by release.
- Parameters:
  - `button` (int): Evdev mouse button code (e.g., 272 / 0x110 for BTN_LEFT).
- Returns: empty

