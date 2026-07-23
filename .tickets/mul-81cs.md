---
id: mul-81cs
status: closed
deps: []
links: []
created: 2026-07-23T02:56:55Z
type: feature
priority: 1
assignee: memgrafter
---
# Add Rust runtime API CLI

Implement an isolated Rust CLI that consumes the FastAPI runtime CRUD HTTP interface, with create/list/get/update/delete commands, API models, errors, tests, and documentation.

## Notes

**2026-07-23T03:00:51Z**

Added isolated cli/ Rust crate with multirepl runtime create/list/get/update/delete commands, HTTP API client, OpenAPI-aligned models, JSON/table output, Cargo lock, docs, and three mock HTTP tests. cargo fmt and cargo metadata passed. Cargo compilation/tests were not run because repository instructions prohibit build commands. Python service contract tests remain 7 passing.

**2026-07-23T03:06:22Z**

User-run cargo test exposed double slash in runtime object URLs; fixing path segment construction.

**2026-07-23T03:06:40Z**

Fixed runtime endpoint URL construction with pop_if_empty() before pushing the ID, eliminating /v1/runtimes//rt_ID. cargo fmt --check passes.

**2026-07-23T03:08:30Z**

Remaining test failure is mock-server SendError when a test intentionally drops the request receiver; make capture send non-fatal.

**2026-07-23T03:08:42Z**

Mock server now treats request-capture delivery as optional and continues writing its HTTP response when the receiver is intentionally dropped. cargo fmt --check passes.

**2026-07-23T03:10:20Z**

User verified `cargo test`: all three HTTP integration tests and all crate/doc test targets pass.
