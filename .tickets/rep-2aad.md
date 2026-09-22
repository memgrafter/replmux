---
id: rep-2aad
status: open
open: true
deps: []
links: [rep-icbz, rep-ao5x]
created: 2026-09-18T17:46:16Z
type: bug
priority: 1
assignee: memgrafter
tags: [claude-code, mcp, battery]
---
# MCP server serializes all kernels; one stuck kernel blocks every other call including repl-manage

Found in the Claude Code battery (rep-icbz). While one kernel was running an endless loop, a `repl` call to a different, idle kernel did not run until the stuck kernel was deleted: sent 10:40:16, ran 10:40:46, the second the delete landed. So the MCP server handles one execution at a time across all kernels, and a single runaway blocks every workspace, including `repl-manage` calls that would let an agent recover.

Proposal: handle requests concurrently, at least one in-flight execution per kernel, and never let `repl-manage` (list, delete, interrupt) wait behind an execution. Per-kernel locks are fine; a global one is not. Add an optional per-call execution timeout on `repl` (server side, default a few minutes, matching the existing "timed out waiting for REPL execution" path) that returns an error but leaves the kernel alive. Test: start `while True: pass` on kernel A, then `repl` on kernel B and `repl-manage list` must return immediately; `delete A` must work while A is spinning.

## Notes

**2026-09-18T17:48:05Z**

PROPOSED FIX (after reading cli/src/mcp.rs, cli/src/broker.rs and cli/src/kernel.rs).

Root cause: `McpServer::serve` (mcp.rs ~L38-55) is a single loop: read one stdin line, `self.handle(...)` synchronously, write the reply. `handle` -> `dispatch` -> `KernelManager::execute` -> `execute_socket`, which blocks on `read_to_end` with `EXECUTION_TIMEOUT` (DEFAULT_OPERATION_TIMEOUT, ~5 min). So while one `repl` waits on a stuck kernel, the loop never reads the next request; every tool call, for any kernel, and every `repl-manage`, queues behind it. The worker itself is already per-kernel (each kernel is its own process with its own socket), so the fix is entirely in the MCP server.

Fix:
1. Make request handling concurrent. `McpServer` fields are all cloneable; wrap the server in `Arc`, and in `serve` spawn a thread per incoming request (`std::thread::spawn` is fine at this request rate; a bounded pool is optional) that runs `handle` and writes its reply through a shared `Arc<Mutex<Stdout>>`, one JSON line per lock. JSON-RPC responses may arrive out of order because they carry the request `id`; Claude Code and the Pi client already match on `id`.
2. Keep executions per kernel in order without a global lock: a `Mutex<HashMap<String, Arc<Mutex<()>>>>` keyed by kernel name, taken only for `repl` calls. `repl-manage` (list, connect, delete, and the new interrupt from rep-xdgn) never takes it, so `delete` and `interrupt` work while that kernel is busy. Delete must then also unblock the waiting `repl` call: it does today, because closing the worker closes the socket and `read_to_end` returns.
3. Add a per-call wait limit that is not destructive: an optional `timeout_secs` on the `repl` tool (default from `--exec-timeout` on `replmux mcp`, default 300 s to match today). On expiry the call returns an error (`still running after 300 s; the kernel keeps running; interrupt or delete it`), the kernel is left alone, and the per-kernel lock is released only when the worker actually answers (tracked by a small "busy" flag so the next call on that kernel reports "kernel busy" instead of silently queueing for minutes).
4. Handle `notifications/cancelled` from the client (Claude Code sends it on TaskStop): look up the in-flight request id and, if rep-xdgn is in, interrupt that kernel; otherwise just drop the reply. Today the cancel is ignored and the worker keeps spinning.

Tests (battery cases, all through the MCP protocol): start `while True: pass` on A; `repl` on B and `repl-manage list` return within a second; `repl-manage delete A` returns while A spins and the A call fails with a clear error; two `repl` calls on the same kernel issued together still run in order; `timeout_secs: 2` on a 5 s sleep returns the timeout error and a later call on that kernel reports busy until the sleep ends. Existing tests in mcp.rs construct the server without a kernel, so the concurrency wiring can be unit-tested with a fake dispatch behind a trait or a function pointer.
