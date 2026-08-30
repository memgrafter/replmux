---
id: rep-iv4j
status: open
open: true
deps: []
links: []
created: 2026-08-30T14:20:51Z
type: bug
priority: 1
assignee: memgrafter
---
# Bubble up kernel worker startup stderr in CLI errors

Kernel create/create_from_kernelspec nulled the worker stderr, so startup failures (e.g. missing pyzmq) surfaced only as 'exited during startup with exit status: 1'. Capture worker stderr to a per-kernel log file and append it (trimmed, capped) to the error message.
