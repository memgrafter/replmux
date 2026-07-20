---
name: jupyter-repl
description: Give agents persistent Python environments across turns. Start, connect to, and manage Jupyter kernels with a CLI and a single-file client (pyzmq only).
---

Start a named kernel in one command, connect from Python code in another — no heavy dependencies, no config cruft.

## Why use it

- **Persistent state across turns** — variables, imports, and execution history survive between agent interactions
- **Zero cognitive overhead** — `jupyter-repl create my-agent` is all the setup needed; the client is one import
- **Multiple isolated environments** — each agent gets its own kernel namespace; no interference
- **Only pyzmq required** — no tornado, traitlets, jupyter_core pulling in 30+ transitive deps

## When to use

- An agent needs to run Python code and keep state between turns (variables, imports, results)
- You need rich output (mime bundles: text/html, image/png) not just stdout
- Multiple agents each need their own isolated Python environment
- You want tab completion, inspection, or interrupt capability

## When NOT to use

- You only need a one-off subprocess call — `subprocess.run` is simpler
- You need GPU access or specialized compute — a kernel won't help with resource allocation
- The code needs to run in a specific virtual environment with custom packages — you'd need to install those into the kernel's env first

## Usage

### CLI (kernel lifecycle)

```bash
jupyter-repl create <name>       # start a named kernel
jupyter-repl list                # show running kernels + status
jupyter-repl connect <name>      # print connection JSON
jupyter-repl delete <name>       # shut down a kernel
```

### Python client

```python
from jupyter_repl import KernelClient
import json

# Connect via CLI output
import subprocess
conn = json.loads(subprocess.run(
    ['jupyter-repl', 'connect', 'my-agent'],
    capture_output=True, text=True
).stdout)
client = KernelClient(conn)

# Or connect from a connection file directly
with open('/path/to/kernel.json') as f:
    conn = json.load(f)
client = KernelClient(conn)

# Execute code — blocks until done, returns (reply, iopub_messages)
reply, outputs = client.execute("print(1+1)", timeout=30)

# Rich output — kernels return mime bundles
reply, outputs = client.execute("x = [1,2,3]; x")
for msg in outputs:
    if msg["msg_type"] == "execute_result":
        print(msg["content"]["data"]["text/plain"])

# Other protocol messages
client.complete("pri", timeout=5)       # tab completion
client.inspect("print", detail_level=1)  # object inspection
client.kernel_info(timeout=5)           # kernel info
client.interrupt()                      # interrupt running code
client.shutdown()                       # graceful shutdown

# Heartbeat
client.start_heartbeat()
client.is_alive()  # True if responsive
client.stop_heartbeat()

client.close()  # clean up sockets
```

## Examples

### Agent with persistent state across turns

```bash
# Turn 1: create kernel and set up environment
jupyter-repl create analysis-bot
python -c "
from jupyter_repl import KernelClient
import json, subprocess
conn = json.loads(subprocess.run(['jupyter-repl', 'connect', 'analysis-bot'], capture_output=True, text=True).stdout)
c = KernelClient(conn)
c.execute('import pandas as pd')
c.close()
"

# Turn 2: use the same kernel with state intact
python -c "
from jupyter_repl import KernelClient
import json, subprocess
conn = json.loads(subprocess.run(['jupyter-repl', 'connect', 'analysis-bot'], capture_output=True, text=True).stdout)
c = KernelClient(conn)
r, o = c.execute('df = pd.DataFrame({\"a\": [1,2]}); df')
for m in o:
    if m['msg_type'] == 'execute_result':
        print(m['content']['data']['text/plain'])
c.close()
"

# Cleanup
jupyter-repl delete analysis-bot
```

### Multiple agents, isolated environments

```bash
jupyter-repl create agent-alpha
jupyter-repl create agent-beta
# Each has its own namespace — no shared state
jupyter-repl list
# NAME               PID        STATUS
# agent-alpha        12345      running
# agent-beta         12346      running
```

## Output

- **CLI**: plain text for `list`, JSON for `connect`, status messages for `create`/`delete`
- **Python client**: `(reply, iopub_messages)` tuples — reply is a protocol message dict, iopub_messages is a list of stream/display_data/status dicts

## How it works (brief)

1. CLI starts `minimal_kernel_clean.py` as a subprocess, writes its connection file to `~/.jupyter-repl/kernels/<name>.json`
2. Client reads that file, creates ZMQ sockets, and speaks the Jupyter messaging protocol
3. Connection files carry 5 ports + HMAC key — the client signs messages with it

**Cost**: one subprocess per kernel (lightweight). **Benefit**: persistent state, rich output, full protocol access with only pyzmq as a dependency.
