---
id: mul-otdh
status: closed
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

**2026-07-23T05:23:11Z**

Completed in current session: exercised create/list/connect/duplicate/delete/recreate, direct and broker-routed repl execution, persistent state, stdout/stderr/errors, Jupyter execute/kernel_info/heartbeat/completion/inspection/completeness, extension reload and swapped-binary local/served routes. Completion support was added and lifecycle suites pass.
