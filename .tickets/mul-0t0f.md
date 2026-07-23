---
id: mul-0t0f
status: closed
deps: []
links: []
created: 2026-07-23T03:33:38Z
type: task
priority: 1
assignee: memgrafter
---
# Test full repl tool lifecycle

## Notes

**2026-07-23T03:34:40Z**

Exercised repl-manage create/list/connect/duplicate-create/delete/recreate and missing-kernel failures. Exercised repl exec/eval, persistent state, functions, stdout/stderr, exceptions, recovery after errors, concurrent serialized calls, post-delete failure, and namespace reset after recreation. All behavior matched expectations.
