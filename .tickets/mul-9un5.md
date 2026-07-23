---
id: mul-9un5
status: open
deps: [mul-2bti]
links: []
created: 2026-07-23T05:24:45Z
type: feature
priority: 3
assignee: memgrafter
---
# Add musl release targets

## Notes

**2026-07-23T05:24:54Z**

Add x86_64-unknown-linux-musl and aarch64-unknown-linux-musl after GNU/macOS automation is stable. Validate cross-compilation of bundled libzmq and static C++ runtime using cross or Zig, then run lifecycle tests in target-compatible environments.
