---
id: mul-5doz
status: closed
deps: []
links: []
created: 2026-07-23T04:41:15Z
type: feature
priority: 0
assignee: memgrafter
---
# Add local circuit-breaker and Unix broker mode

## Notes

**2026-07-23T04:44:30Z**

Implemented same-binary `serve` mode and shared kernel service dispatch. Kernel CLI defaults to auto transport: one Unix connect attempt routes to broker when active, only ENOENT/ECONNREFUSED short-circuit in-process, and permission/protocol/other failures remain errors. Added explicit local/socket modes, 0600 socket, stale cleanup, bounded I/O, concurrent handling, docs, circuit unit tests, and served lifecycle integration test. Formatting/metadata/diff checks pass; build/tests pending user run.

**2026-07-23T04:45:25Z**

First build found dispatch Socket arm returned BrokerClientError while Local/Auto returned String. Mapped unavailable/failure variants to explicit String errors for required socket mode. Formatting and diff checks pass; awaiting rerun.

**2026-07-23T04:46:17Z**

User verification passed: 7 FastAPI tests, 6 Rust unit tests, 3 API client tests, 2 kernel lifecycle tests including served Unix-socket lifecycle, and doc tests.
