---
id: mul-h82e
status: closed
deps: []
links: []
created: 2026-07-23T05:20:16Z
type: task
priority: 1
assignee: memgrafter
---
# Verify swapped binary lifecycle routes

## Notes

**2026-07-23T05:22:07Z**

Verified swapped default binary through both lifecycle routes. Broker absent: repl-manage created swapped-local, worker reparented to PID 1, direct repl persisted state and returned 42. Broker active: repl-manage created swapped-served as child of multirepl serve PID 39681, broker socket active, repl persisted state and returned 45. Both kernels deleted; broker, socket, logs, and PID file cleaned.
