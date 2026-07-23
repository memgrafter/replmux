---
id: mul-6kt3
status: closed
deps: []
links: []
created: 2026-07-23T03:18:27Z
type: task
priority: 2
assignee: memgrafter
---
# Add build and test script

## Notes

**2026-07-23T03:19:06Z**

Added scripts/build-and-test.sh. It validates uv/cargo, syncs the locked service dev environment, runs service tests, then builds and tests the Rust CLI in release mode with locked dependencies. bash -n passed; build/test commands were not executed per repository instructions.
