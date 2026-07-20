---
id: mul-otdh
status: open
deps: []
links: []
created: 2026-07-20T04:50:21Z
type: task
priority: 1
assignee: memgrafter
tags: [testing, repl]
---
# Exercise full pi REPL lifecycle

Review pi/extension/replTool.ts and its Python dependencies, then run lifecycle and varied use-case tests; record failures and fixes if needed.

## Notes

**2026-07-20T04:52:17Z**

Initial lifecycle run required recreating the moved .venv; after installing pyzmq, create/list/connect/duplicate/delete and basic direct/client execution worked, but complete_request timed out because minimal kernel has no complete handler. Reproducible environment setup is needed.
