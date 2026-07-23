---
id: rep-b7wx
status: in_progress
deps: []
links: []
created: 2026-07-23T18:32:01Z
type: task
priority: 2
assignee: memgrafter
---
# Design remote environment provisioning, execution, and lifecycle

Here's the picture on runtime installation, execution, tracking, and uninstallation:

---

## Current state: a local filesystem mess, as you said

### Installation: ad-hoc, three separate paths

| Path | Mechanism | Tracking |
|------|-----------|----------|
| **Minimal Python worker** | Ships alongside the binary in `dist/`. Found by walking CWD → binary ancestors → bare filename. `REPLMUX_KERNEL_SCRIPT` env var. | None. No version check, no integrity check. |
| **Standard kernelspecs** | `kernelspec.rs` searches a hardcoded list of directories: `JUPYTER_PATH`, XDG, `~/Library/Jupyter/`, `~/.local/share/jupyter/`, `/Library/`, `/opt/homebrew/`, `/usr/local/`, `/usr/share/`. | None. Discovery only. No dependency tracking, no version pinning. |
| **Test kernel matrix** | `tests/jupyter-kernels/install-kernels.py` uses micromamba to create prefix environments from `kernels.toml`. | Only `kernels.toml` declares what *should* exist. No actual state tracking of what *is* installed. |

### Execution: two different protocols, no unification

```
Minimal Python worker:  Unix socket, direct JSON RPC
Standard kernelspec:    Jupyter ZMQ (5 ports, HMAC signing)
```

Both work, but the service layer (`service/`) knows nothing about either. It stores `environment.executable = "python3"` as a string — it doesn't launch kernels, track PIDs, or route execution. The **kernel lifecycle is entirely local** in `kernel.rs`.

### Tracking: metadata and reality are completely separate

```
Service (SQLite)           KernelManager (filesystem)
───────────────            ─────────────────────────
runtimes table             ~/.jupyter-repl/kernels/
  id: rt_xxx                 name.json  ← connection file
  name: "analysis"           name.pid   ← raw PID
  status: "idle"             name.sock  ← (unused for Jupyter kernels)
  language: "python"
  worker_generation: 0       (no environment tracking)
  revision: 1               (no version pinning)
```

The service says "runtime exists with status=idle". The filesystem says "kernel is running at PID 12345". **Neither one can verify the other.** If you create a runtime via the API and then create a kernel via the CLI, they are unrelated records.

### Uninstallation: best-effort cleanup

```rust
// kernel.rs::delete
1. Graceful Jupyter shutdown (if Jupyter kernel)
2. SIGTERM the PID
3. Wait 2 seconds
4. SIGKILL if still alive
5. Remove .json, .pid, .sock files
```

No uninstallation of the environment itself (micromamba prefix, packages). No cleanup of the service record. No cascade delete.

---

## What's missing for remote/VM environments

The current architecture assumes everything is local:

1. **`kernel.rs` spawns `Command::new(&self.python)` directly** — only works for local executables
2. **Connection files use `127.0.0.1`** — hardcoded localhost ports
3. **PID tracking uses `libc::kill()`** — only valid on the same host
4. **No SSH, Docker, or VM transport** — `TransportMode` only has `Local`, `Socket`, `Auto`
5. **No remote environment provisioning** — `install-kernels.py` runs locally with micromamba
6. **Service only stores metadata** — no remote endpoint awareness

### What a remote architecture needs

```
Client (CLI / MCP / Pi)
  │
  ├── Runtime API (already exists, needs extension)
  │     POST /v1/environments          ← provision an environment (local, SSH, VM, container)
  │     GET  /v1/environments/{id}      ← check environment readiness
  │     DELETE /v1/environments/{id}    ← tear down
  │
  ├── Kernel broker (needs remote transport)
  │     TransportMode::Remote           ← SSH tunnel, Docker exec, VM API
  │     Connection forwarding           ← proxy ZMQ through SSH/port forwarding
  │
  └── Environment spec (needs declaration)
         kind: "micromamba-prefix"
         target: "local" | "ssh:user@host" | "docker:image" | "vm:template"
         packages: ["python=3.12", "ipykernel"]
         health_check: heartbeat
```

The `service/` has the right shape for this — it already tracks `environment`, `worker_generation`, and `snapshot_policy` — but none of the plumbing connects the metadata to actual kernel lifecycle beyond the local filesystem.
