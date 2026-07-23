---
id: rep-avvb
status: open
deps: []
links: []
created: 2026-07-23T06:24:40Z
type: feature
priority: 1
assignee: memgrafter
---
# Ship a self-contained default kernel runtime

Remove the default runtime dependency on system Python and pyzmq. Investigate shipping an embedded or standalone Python distribution (for example python-build-standalone/PyOxidizer) versus implementing the kernel worker in Rust. Release artifacts should work on supported targets without Python preinstalled; retain an explicit opt-in system-Python mode if useful. Add clean-machine packaging and lifecycle coverage.
