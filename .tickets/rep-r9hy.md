---
id: rep-r9hy
status: open
deps: []
links: [mul-hc2u, rep-sk5z, rep-ao5x]
created: 2026-07-26T22:13:57Z
type: bug
priority: 1
assignee: memgrafter
tags: [pi-extension, timeout, repl]
---
# Make Pi extension timeouts configurable

The Pi extension at pi/extension/replTool.ts hard-codes 30,000 ms at three independent boundaries:

1. runCli() passes timeout: 30_000 to pi.exec for repl-manage lifecycle commands.
2. sendJson() starts a 30_000 ms Unix-socket timer used by broker requests and direct kernel-socket execution.
3. executeViaCli() passes timeout: 30_000 to pi.exec for the CLI/Jupyter fallback execution path.

Impact: normal Pi REPL calls, including legitimate long-running nested model calls, can be terminated at 30 seconds regardless of caller needs. The broker/direct socket and CLI fallback paths should have consistent configurable timeout behavior rather than a fixed cap.

Desired change:
- Replace all three literals with one documented configuration policy.
- Support a configurable execution timeout suitable for long-running calls; retain a safe default.
- Keep lifecycle timeout separately configurable if lifecycle and execution need different policies.
- Preserve AbortSignal cancellation.
- Validate finite positive values and define whether zero means disabled or is rejected.
- Add tests covering broker/direct socket timeout, CLI fallback timeout, lifecycle timeout, and abort behavior.
- Document environment/configuration names and precedence.

Relationship: mul-hc2u introduced the 30-second lifecycle timeout. rep-sk5z proposes asynchronous wait semantics for long-running execution, but configurability is still needed for synchronous execution and lifecycle paths.
