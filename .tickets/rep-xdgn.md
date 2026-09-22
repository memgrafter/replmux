---
id: rep-xdgn
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
# Default worker cannot be interrupted; runaway code is only recoverable by deleting the kernel and its state

Found in the Claude Code battery (rep-icbz). `replmux kernel interrupt <name>` on the default minimal worker fails: "the minimal worker does not support non-destructive interruption; use a standard kernelspec or explicitly delete the kernel". Stopping the tool call on the client side ends the wait but the worker keeps spinning at 100% CPU. The only recovery is `delete`, which throws away the kernel's state, i.e. the whole point of the tool.

Proposal: make the minimal worker interruptible without losing state. Options: run user code on a worker thread and raise `KeyboardInterrupt` in it via `PyThreadState_SetAsyncExc` on interrupt; or run code in a child process per kernel with the namespace held by a supervisor and send SIGINT to the child; or install a SIGINT handler in the worker and have `interrupt` send the signal. Also expose it as a `repl-manage` action (`interrupt`) so an agent can recover without the shell. Test: `while True: pass`, then interrupt; the call returns `KeyboardInterrupt` and a following `repl` call sees earlier variables.

## Notes

**2026-09-18T17:48:05Z**

PROPOSED FIX (after reading cli/assets/python_minimal_kernel.py and cli/src/kernel.rs).

Why it is destructive today: the worker installs `signal.signal(signal.SIGINT, cleanup)` (kernel script ~L600), and `cleanup` sets `kernel.running = False`, so SIGINT ends the process. Executions do not even run on the main thread: `_socket_loop` (~L431) spawns a thread per socket client and `_handle_socket_client` runs `_execute_direct` there, so a Python-level KeyboardInterrupt could not reach the running code anyway. `KernelManager::interrupt` (kernel.rs ~L345) therefore refuses minimal workers up front.

Fix, worker side (the whole change is inside the kernel script):
1. Run user code on ONE dedicated executor thread, not on socket-client threads: replace the per-client `_execute_direct` call with `queue.put((code, future))` and have an executor thread loop `code, fut = queue.get(); fut.set_result(self._execute_direct(code))`. Socket threads just wait on the future. The existing `self._lock` becomes unnecessary for ordering but can stay.
2. Interrupt = raise KeyboardInterrupt inside that executor thread: `ctypes.pythonapi.PyThreadState_SetAsyncExc(ctypes.c_ulong(executor_ident), ctypes.py_object(KeyboardInterrupt))`. It fires at the next bytecode boundary, which covers `while True` loops and any pure-Python work, and `_execute_direct` already catches exceptions (`except Exception` must become `except BaseException` so KeyboardInterrupt is reported as `KeyboardInterrupt` with the namespace intact). Blocking C calls (`time.sleep`, socket reads) return first and are interrupted on return; document that limit.
3. Trigger it two ways: a socket request `{"op": "interrupt"}` handled outside the queue (so it works while the queue is busy), and a real `SIGINT` handler that does the same instead of `cleanup`. Keep SIGTERM as the shutdown signal. `handle_interrupt_request` (~L421, Jupyter path) calls the same function instead of replying ok while doing nothing.

Fix, Rust side:
4. `KernelManager::interrupt`: for a minimal worker, send `{"op":"interrupt"}` over the existing Unix socket (a small sibling of `execute_socket`) instead of returning the error; fall back to SIGINT to the pid from the pid file. Return `InterruptOutcome` like the kernelspec path.
5. `repl-manage` gains `"interrupt"` in its action enum and dispatch (mcp.rs `call_repl_manage`, `tool_definitions`), so an agent can recover without a shell.

Tests (battery cases): start `while True: pass`, call interrupt; the pending `repl` returns `KeyboardInterrupt` within a second and a following `repl` still sees variables set before the loop. `time.sleep(30)` + interrupt returns when the sleep ends, not before, and the kernel survives. Existing rep-ao5x (non-destructive wait timeout) becomes a follow-on: with an interruptible worker the server can time out a call by interrupting instead of abandoning it.
