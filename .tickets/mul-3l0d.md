---
id: mul-3l0d
status: closed
deps: []
links: []
created: 2026-07-23T04:12:52Z
type: task
priority: 1
assignee: memgrafter
---
# Test released Rust CLI binary lifecycle

## Notes

**2026-07-23T04:13:22Z**

Tested the binary extracted from the packaged aarch64 macOS release archive without changing the extension. Verified create/list/connect, persistent exec/eval, stdout/stderr, Python error exit, duplicate rejection, delete/missing rejection, recreate with clean namespace, sibling kernel-script discovery, and cleanup. Packaged binary lifecycle passed.
