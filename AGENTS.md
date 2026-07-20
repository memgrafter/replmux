## Jupyter REPL Client — Minimal Extract from jupyter_client (~300 lines, deps: pyzmq only)### What it is`jupyter_repl.py` is a single-file Jupyter protocol client extracted from `jupyter_client` (which pulls in tornado, traitlets, jupyter_core, python-dateutil). It connects to any running Jupyter kernel (ipykernel, xeus-kernel, IRkernel, julia-kernel, etc.) and lets you execute code, get rich output, interrupt, tab-complete, inspect, and monitor heartbeat.### Quick start```python
from jupyter_repl import KernelClient
import json

# Connect to a running kernel via its connection file
with open("/tmp/kernel-123.json") as f:
    conn = json.load(f)
client = KernelClient(conn)

# Execute code, get reply + IOPub output
reply, outputs = client.execute("print(1+1)", timeout=30)
print(outputs)  # [{"msg_type": "stream", "content": {"name": "stdout", "text": "2\n"}}, ...]

# Rich display data (e.g., a DataFrame)
reply, outputs = client.execute("pd.DataFrame({'a': [1,2]})", timeout=30)
for msg in outputs:
    if msg["msg_type"] in ("execute_result", "display_data"):
        print(msg["content"]["data"].keys())  # dict_keys(['text/plain', 'text/html', ...])

# Interrupt running code
client.interrupt()

# Tab completion
reply = client.complete("pri", timeout=5)
print(reply["content"]["matches"])

# Inspect object
reply = client.inspect("print", detail_level=1, timeout=5)
print(reply["content"]["docstring"])

# Heartbeat check
client.start_heartbeat()
print(client.is_alive())  # True if kernel is responsive
client.stop_heartbeat()

# Shutdown
client.shutdown(restart=False)
```
### API surface- **`KernelClient(conn_info)`** — constructor; `conn_info` is a dict from the kernel's JSON connection file (keys: `shell_port`, `iopub_port`, `stdin_port`, `control_port`, `hb_port`, `ip`, `key`, `transport`, `signature_scheme`).
- **`execute(code, *, silent=False, store_history=True, user_expressions=None, allow_stdin=True, stop_on_error=True, timeout=30)`** — returns `(reply, iopub_messages)`. Blocks until execution completes.
- **`interrupt()`** — sends `interrupt_request` on control channel.
- **`complete(code, cursor_pos=None, timeout=30)`** — tab completion request/reply.
- **`inspect(code, cursor_pos=None, detail_level=0, timeout=30)`** — object inspection.
- **`kernel_info(timeout=30)`** — kernel info request/reply.
- **`is_complete(code, timeout=5)`** — is the code complete and ready to execute?
- **`shutdown(restart=False, timeout=10)`** — graceful shutdown via control channel.
- **`start_heartbeat() / stop_heartbeat()`** — start/stop heartbeat thread.
- **`is_alive()`** — returns `True` if heartbeat is beating.
- **`close()`** — close all sockets and context.

### Design notes- No managers, no launchers, no provisioners — you connect to an *already-running* kernel.
- The connection file schema is standard Jupyter: 5 ports + HMAC key + signature scheme.
- HMAC signing uses `hmac-sha256` by default (same as jupyter_client). Set `key=b""` in conn_info to disable (insecure).
- The heartbeat runs in a daemon thread (like the original HBChannel), not an event loop.
- IOPub messages are collected during `execute()` until the `status: idle` message arrives.
- The reply is matched by `parent_header.msg_id` to ensure correct request/reply pairing.

### What was left out (and why)- **Kernel lifecycle** — start/stop kernels is a manager concern, not a client concern.
- **Protocol version adaptation** — protocol v5.4 is the current standard; adaptation adds complexity for minimal benefit.
- **SSH tunneling** — if you need SSH, tunnel at the network layer.
- **Auto-restart** — add it in your own code if needed.
- **Threaded client** — blocking is simpler and sufficient for agent use.
- **orjson/msgpack serialization** — standard `json` is fast enough and has zero deps.

### Testing with a real kernel```bash
# Start a kernel and note the connection file path
jupyter console --kernel python3 --existing
# or in Python:
python -c "from jupyter_client import KernelManager; km = KernelManager(); km.start_kernel(); print(km.connection_file)"
# Then use jupyter_repl.py to connect to it
```
### File structure```njupyter_repl.py   # ~300 lines, single file, deps: pyzmq only
README.md        # human-facing docs
AGENTS.md        # agent-facing instructions (this file)
```

</project_instructions>