# jupyter_repl — Minimal Jupyter REPL Client

A single-file (~300 lines), zero-dependency (except pyzmq) Jupyter protocol client. Connect to any running Jupyter kernel and execute code, get rich output, interrupt, tab-complete, inspect, and monitor heartbeat.

Extracted from [jupyter_client](https://github.com/jupyter/jupyter_client) which pulls in ~800KB of transitive dependencies (tornado, traitlets, jupyter_core, python-dateutil). This keeps only the protocol wiring — no managers, launchers, provisioners, or config cruft.

## Installation

```bash
pip install pyzmq
# That's it.
```

## Quick Start

```python
from jupyter_repl import KernelClient
import json

# Connect to a running kernel via its connection file
with open("/tmp/kernel-123.json") as f:
    conn = json.load(f)
client = KernelClient(conn)

# Execute code — blocks until done, returns (reply, iopub_messages)
reply, outputs = client.execute("print(1+1)", timeout=30)
for msg in outputs:
    print(msg["msg_type"], msg["content"])
# stream {'name': 'stdout', 'text': '2\n'}
# status {'execution_state': 'idle'}

# Rich output — kernels return mime bundles (text/plain, text/html, image/png, etc.)
reply, outputs = client.execute("import pandas as pd; pd.DataFrame({'a': [1,2]})")
for msg in outputs:
    if msg["msg_type"] == "execute_result":
        print(msg["content"]["data"].keys())
# dict_keys(['text/plain', 'text/html', ...])

client.close()
```

## API

### `KernelClient(conn_info)`

Constructor. `conn_info` is a dict from the kernel's JSON connection file:

| Key | Description |
|---|---|
| `shell_port` | Port for request/reply messages |
| `iopub_port` | Port for broadcast output (stdout, display data) |
| `stdin_port` | Port for input requests |
| `control_port` | Port for interrupt/shutdown |
| `hb_port` | Port for heartbeat |
| `ip` | Kernel IP address (usually `127.0.0.1`) |
| `key` | HMAC key for message signing |
| `transport` | Transport protocol (`tcp` or `ipc`) |
| `signature_scheme` | Signature scheme (default: `hmac-sha256`) |

### Methods

- **`execute(code, *, timeout=30)`** — Execute code in the kernel. Returns `(reply, iopub_messages)`. Blocks until `status: idle` is received.
- **`interrupt()`** — Interrupt running code via control channel.
- **`complete(code, cursor_pos=None, timeout=30)`** — Tab completion. Returns reply dict with `content.matches`.
- **`inspect(code, cursor_pos=None, detail_level=0, timeout=30)`** — Object inspection. Returns reply with docstring/signature.
- **`kernel_info(timeout=30)`** — Kernel info. Returns reply with language name/version/protocol version.
- **`is_complete(code, timeout=5)`** — Check if code is syntactically complete.
- **`shutdown(restart=False, timeout=10)`** — Graceful shutdown via control channel.
- **`start_heartbeat()` / `stop_heartbeat()`** — Start/stop heartbeat monitoring thread.
- **`is_alive()`** — Returns `True` if the kernel's heartbeat is responding.
- **`close()`** — Close all sockets and destroy the zmq context.

## Why this exists

[jupyter_client](https://github.com/jupyter/jupyter_client) is ~19K LOC with heavy dependencies. Most of it is kernel lifecycle management (starting, stopping, restarting kernels). If you just need to *connect* to an already-running kernel and execute code, you need maybe 300 lines — message framing, HMAC signing, socket I/O, and request/reply matching.

This is that 300 lines. No managers, no provisioners, no SSH tunnels, no protocol adaptation, no config system.

## What it works with

Any kernel implementing the Jupyter messaging protocol:
- **Python**: ipykernel
- **C++**: xeus-cling
- **R**: IRkernel
- **Julia**: IJulia
- **JavaScript**: zshim
- **And many more**

The kernel doesn't know or care whether you're a notebook, VSCode, or an agent.

## Example: Connect to a running kernel

```python
import json
from jupyter_repl import KernelClient

# Find the connection file (jupyter writes it when starting a kernel)
conn_file = "/tmp/kernel-12345.json"
with open(conn_file) as f:
    conn = json.load(f)

client = KernelClient(conn)
client.start_heartbeat()

if not client.is_alive():
    raise RuntimeError("Kernel is not responding")

# Run code and collect output
reply, outputs = client.execute("x = 42; print(x)")
print(f"Execution status: {reply['content']['status']}")
for msg in outputs:
    if msg["msg_type"] == "stream":
        print(msg["content"]["text"], end="")

# Cleanup
client.shutdown()
client.close()
```

</project_instructions>
