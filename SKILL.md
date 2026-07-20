---
name: multirepl
description: Multiplayer Python for agents. Persistent Jupyter kernels with a pi extension — zero subprocess overhead per call, shared namespaces, full protocol access.
---

Give agents persistent Python environments across turns via Jupyter kernels. Managed by a pi extension — `repl` for code execution, `repl-manage` for lifecycle. Shared namespaces for multiplayer, isolated per kernel.

## Why use it

- **Persistent state across turns** — variables, imports, execution history survive between agent interactions
- **Zero subprocess overhead** — `repl` talks to the kernel directly over a Unix socket (no Python per call)
- **Multiplayer** — multiple agents share the same kernel namespace, read/write state freely
- **Minimal kernel** — lightweight Python kernel (~300 lines, pyzmq only), no ipykernel bloat
- **Full Jupyter protocol** — rich output, mime bundles, tab completion, inspection, interrupt

## When to use

- An agent needs to run Python code and keep state between turns
- Multiple agents need to share a Python environment (multiplayer)
- You need rich output (DataFrames, plots) not just stdout
- You want a lightweight kernel without ipykernel's 30+ transitive deps

## When NOT to use

- You only need a one-off subprocess call — `subprocess.run` is simpler
- You need GPU access or specialized compute — a kernel won't help with resource allocation
- You need full ipykernel features (magics, async, debugging) — use ipykernel instead

## Usage

### pi extension tools

**Execute code** (zero subprocess):
```
repl { code: "x = 42", name: "pri-20260720013947" }
→ pri-20260720013947: x = 42 (ok)

repl { code: "x", name: "pri-20260720013947" }
→ pri-20260720013947: x → 42
```

**Manage kernels**:
```
repl-manage { action: "create" }                  → auto-generates name (pri-20260720013947)
repl-manage { action: "create", name: "my-kernel" } → explicit name
repl-manage { action: "list" }                    → table of running kernels
repl-manage { action: "connect", name: "my-kernel" } → connection JSON
repl-manage { action: "delete", name: "my-kernel" } → shuts down
```

### CLI (standalone)

```bash
jupyter-repl create <name>       # start a named kernel
jupyter-repl list                # show running kernels
jupyter-repl connect <name>      # print connection JSON
jupyter-repl delete <name>       # shut down a kernel
```

### Python client library

```python
from jupyter_repl import KernelClient
import json

with open("~/.jupyter-repl/kernels/my-kernel.json") as f:
    conn = json.load(f)
client = KernelClient(conn)

reply, outputs = client.execute("x = 42; x", timeout=30)
for msg in outputs:
    if msg["msg_type"] == "execute_result":
        print(msg["content"]["data"]["text/plain"])
client.close()
```

## Architecture

```
Agent A ──┐
Agent B ──┼── Unix socket (JSON) ──→ minimal_kernel_clean.py ──→ Python namespace
Agent C ──┘                          (ZMQ + socket server)

repl-manage ──→ jupyter_repl_cli.py ──→ spawns/manages kernels
```

- **Kernel** (`minimal_kernel_clean.py`): Jupyter protocol over ZMQ + direct JSON over Unix socket. Persistent Python namespace.
- **CLI** (`jupyter_repl_cli.py`): Lifecycle management. Spawns kernels, tracks PIDs, reads connection files.
- **Client** (`jupyter_repl.py`): ~300 line Jupyter protocol client. Used by CLI for graceful shutdown.
- **Extension** (`pi/extension/replTool.ts`): pi tools. `repl` talks kernel socket directly. `repl-manage` calls CLI.

## Multiplayer model

One kernel, multiple agents, shared namespace. Agents read/write freely — coordination is the agent's responsibility, not the kernel's.

```
Agent A: repl { code: "x = 42", name: "shared" }
Agent B: repl { code: "x", name: "shared" }     → 42
Agent B: repl { code: "x = 99", name: "shared" } → (ok)
Agent A: repl { code: "x", name: "shared" }      → 99
```

## Output format

- **repl call**: `repl: <kernel-name>` with code prefixed by `>>>` / `...`
- **repl result**: `→ <value>` (expressions), `(ok)` (exec), `✗ <error>` (errors)
- **repl-manage**: plain text status, JSON for connect

## File structure

```
jupyter_repl_cli.py     # CLI: create/list/connect/delete
jupyter_repl.py         # Client: ~300 lines, pyzmq only
minimal_kernel_clean.py # Kernel: ZMQ + Unix socket server
shared_repl_socket.py   # DEAD — replace with kernel's built-in socket
pi/extension/           # pi extension (symlink to replTool.ts)
```