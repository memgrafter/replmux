---
id: rep-ao5x
status: open
deps: []
links: [rep-sk5z, rep-r9hy]
created: 2026-08-17T20:36:22Z
type: feature
priority: 1
assignee: memgrafter
tags: [repl, timeout, async, lifecycle]
---
# Make REPL execution timeout a non-destructive wait timeout

Execution timeout must be a non-destructive wait timeout, not a kernel termination mechanism.

Current behavior has synchronous 30-second limits in the Pi extension, broker, CLI, direct Unix socket, and Jupyter execution paths. Long-running work must remain alive after a client stops waiting. Kernel termination remains an explicit repl-manage delete operation.

Desired behavior:
- Assign each execution a request ID before work begins.
- Run execution independently of the requesting client connection.
- On wait timeout, return a running/pending status with kernel name and request ID; do not interrupt, terminate, or delete the kernel.
- Retain completion status and result server-side for later retrieval.
- Add wait/status/result operations and define result retention/cleanup behavior.
- Optionally support an explicit soft interrupt operation, distinct from repl-manage delete.
- Preserve AbortSignal semantics as cancellation of the client wait unless an explicit interrupt is requested.
- Keep startup, shutdown, lifecycle, and transport-connect timeouts separately bounded.
- Add persistent-state tests proving work completes after the initial caller times out/disconnects and its result can be retrieved later.
- Add failure tests for unknown request IDs, execution errors, disconnects, and kernel deletion while work is pending.

Do not solve this solely by increasing timeout literals or closing the socket and discarding the eventual response.

## Notes

**2026-08-17T20:38:26Z**

Interim mitigation: raised synchronous operation/request/I/O timeouts from 30s to 300s and unified constants within the Pi extension and Rust CLI scopes. This does not implement the ticket's non-destructive asynchronous wait semantics.

**2026-08-18T12:06:18Z**

Timeout test results (2026-08-18, 300s limits): (1) direct-socket path times out at exactly 300s with the mapped message 'timed out waiting for REPL execution'; (2) broker-client path (broker.rs send_request) times out at 300s but prints raw 'Resource temporarily unavailable (os error 35)' — the read-timeout error is not mapped to a friendly message (macOS SO_RCVTIMEO surfaces as EAGAIN); (3) the kernel process survives both timeouts (non-destructive), but the orphaned execution keeps holding the worker's execution lock, so subsequent execs queue behind it until the orphan finishes (observed ~10s of extra blocking); (4) Jupyter path could not be live-tested: both installed kernelspecs are broken (Julia binary deleted, no ijskernel) and ipykernel is not installed; Rust test suite (6 tests) passes.
