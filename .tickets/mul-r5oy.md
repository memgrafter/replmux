---
id: mul-r5oy
status: closed
deps: [mul-7jmv]
links: []
created: 2026-07-23T03:40:32Z
type: feature
priority: 1
assignee: memgrafter
---
# Port REPL lifecycle and execution into Rust CLI

## Notes

**2026-07-23T03:44:12Z**

Implemented Rust kernel lifecycle and Unix-socket REPL execution in cli/: nested `kernel` commands plus hidden top-level Python CLI compatibility commands, PID/connection cleanup, startup/shutdown timeouts, JSON output, docs, unit tests, and end-to-end lifecycle test. Formatting and git diff checks passed. Build/tests were not run per repository instruction, so ticket remains in progress pending user verification with scripts/build-and-test.sh.

**2026-07-23T03:45:33Z**

First release lifecycle test exposed macOS SUN_LEN failure from excessively long temporary socket path. Shortened test directory/name and added a Drop cleanup guard to prevent leaked kernels on future assertion failures. Removed leaked PID 52776 and its test directory. Awaiting rerun.
