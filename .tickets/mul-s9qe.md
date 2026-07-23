---
id: mul-s9qe
status: closed
deps: []
links: []
created: 2026-07-23T03:35:18Z
type: task
priority: 1
assignee: memgrafter
---
# Assess Rust CLI parity with Python CLI

## Notes

**2026-07-23T03:35:37Z**

Assessment: cli/ implements runtime metadata CRUD only. It has 0/4 Python kernel lifecycle commands and no Unix-socket repl execution or Jupyter control protocol. Runtime CRUD is a separate Rust-only surface. Remaining lift includes kernel lifecycle module, connection/PID handling, Unix socket client/framing, CLI command surface, cleanup/error parity, and lifecycle integration tests.
