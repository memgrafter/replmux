---
id: mul-hc2u
status: closed
deps: []
links: []
created: 2026-07-23T04:17:51Z
type: feature
priority: 1
assignee: memgrafter
---
# Switch Pi extension lifecycle to Rust CLI

## Notes

**2026-07-23T04:19:29Z**

Switched repl-manage from Python+jupyter_repl_cli.py execution to the Rust multirepl binary via pi.exec with abort signal and 30-second timeout. Added binary parameter and MULTIREPL_BINARY override; retained direct Unix-socket repl transport. Extension load test passed with pi --list-models.

**2026-07-23T04:23:36Z**

Live extension test initially found pi closure was unavailable because replManageTool was module-scoped. Converted it to a factory that captures ExtensionAPI. After /reload, explicit release-binary create/list/connect succeeded, direct repl execution preserved state/output, and Rust-backed delete shut down the test kernel.
