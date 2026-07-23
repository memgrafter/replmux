---
id: mul-ebhb
status: closed
deps: []
links: []
created: 2026-07-23T04:47:15Z
type: feature
priority: 1
assignee: memgrafter
---
# Route Pi REPL through optional Rust broker

## Notes

**2026-07-23T04:48:11Z**

Updated Pi repl execution to attempt ~/.multirepl/b.sock (or MULTIREPL_BROKER_SOCKET) first using the Rust broker wire protocol. Only ENOENT/ECONNREFUSED short-circuit to the existing direct kernel socket; broker permission/protocol/service failures remain errors. Socket reads now accumulate complete JSON responses instead of assuming one data chunk. Extension load/diff checks pass; live broker and fallback verification pending /reload.

**2026-07-23T04:50:14Z**

Live verification after /reload passed both routes. With broker absent, repl persisted/evaluated through direct kernel fallback. With multirepl serve active, lsof observed the broker process holding an accepted Unix connection during a long repl call and execution state persisted. After stopping broker while stale socket remained, ECONNREFUSED correctly short-circuited back to kernel. Test kernel, broker, socket, logs, and PID file cleaned up.
