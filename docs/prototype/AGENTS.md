## replmux — Multiplayer Python for Agents

### What it is

A Jupyter kernel system built for agent use. Persistent Python environments, shared namespaces for multiplayer, zero subprocess overhead per call. Replaces the 30+ transitive deps of `jupyter_client` with pyzmq-only components.

### Architecture

```
pi extension (replTool.ts)
├── repl: executes code via Unix socket → minimal_kernel_clean.py
└── repl-manage: manages kernels via CLI → jupyter_repl_cli.py

minimal_kernel_clean.py
├── ZMQ channels (shell, iopub, control, heartbeat) — Jupyter protocol
├── Unix socket server — direct JSON access for extension
└── Persistent Python namespace — shared across all callers

jupyter_repl_cli.py
├── create: spawns kernel, waits for connection file, writes PID
├── list: scans ~/.jupyter-repl/kernels/ for .json files
├── connect: reads and prints <name>.json
└── delete: graceful shutdown via jupyter_repl.py + fallback kill

jupyter_repl.py (~300 lines)
└── KernelClient: Jupyter protocol client (execute, interrupt, complete, inspect, shutdown)
```

### Key design decisions

- **No subprocess per execute** — extension talks kernel Unix socket directly (JSON in/out)
- **Explicit kernel names** — `repl` requires `name` param, no implicit "active kernel"
- **Multiplayer by default** — shared namespace, agents coordinate themselves
- **Minimal kernel, not ipykernel** — no magics, no async, no rich display hooks. exec/eval in a persistent dict.
- **Expression vs exec** — AST dispatch: single expressions use eval (return value), statements use exec (no value)
- **RLock, not Lock** — socket handler calls do_execute which also locks, needs reentrant lock

### File roles

| File | Role | Deps |
|------|------|------|
| `minimal_kernel_clean.py` | Kernel: ZMQ + socket server, persistent namespace | pyzmq |
| `jupyter_repl_cli.py` | CLI: lifecycle management | stdlib + jupyter_repl (import) |
| `jupyter_repl.py` | Client: Jupyter protocol (~300 lines) | pyzmq |
| `shared_repl_socket.py` | **DEAD** — kernel has built-in socket now | — |
| `pi/extension/replTool.ts` | pi extension: repl + repl-manage tools | pi-coding-agent, pi-tui |

### Kernel socket protocol

The kernel's Unix socket (`~/.jupyter-repl/kernels/<name>.sock`) speaks JSON:

**Request**: `{ "code": "x = 42" }`
**Response**: `{ "ok": true, "mode": "exec|eval", "code": "...", "result": "42", "stdout": "", "stderr": "", "error": null }`

- `mode: "eval"` — single expression, result is repr of value
- `mode: "exec"` — statements, result is null
- `error` set on exception, `ok` is false
- Socket path stored in connection JSON: `socket_path` field

### Extension rendering

The extension defines `renderCall` and `renderResult` for TUI display:

- **Call**: `repl: <name>` + code with `>>>`/`...` prefixes (Python REPL style)
- **Result**: `→ <value>` (eval), `(ok)` (exec), `✗ <error>` (errors)
- Uses `_onUpdate` to send label early before args are fully parsed

### Connection file format

`~/.jupyter-repl/kernels/<name>.json`:
```json
{
  "shell_port": 60462,
  "iopub_port": 60463,
  "control_port": 60464,
  "hb_port": 60465,
  "stdin_port": 0,
  "ip": "127.0.0.1",
  "key": "hex-encoded-hmac-key",
  "transport": "tcp",
  "signature_scheme": "hmac-sha256",
  "kernel_name": "python3",
  "socket_path": "/path/to/kernels/<name>.sock"
}
```

### Future: Rust rewrite

Ticket `pri-fs68` — single Rust binary replaces CLI + client. Kernel stays Python (or gets its own Rust rewrite later). ZMQ via `zmq-build` (static libzmq, no system dep).

### Testing

```bash
# Start kernel
jupyter-repl create test-kernel

# Execute via Python client
python -c "
from jupyter_repl import KernelClient
import json
with open('~/.jupyter-repl/kernels/test-kernel.json') as f:
    conn = json.load(f)
c = KernelClient(conn)
r, o = c.execute('x = 42; x')
print(r, o)
c.close()
"

# Cleanup
jupyter-repl delete test-kernel
```