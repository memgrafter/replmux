---
id: rep-l68y
status: closed
deps: []
links: []
created: 2026-07-23T06:34:07Z
type: feature
priority: 2
assignee: memgrafter
---
# Add fast release mode without clean rebuild

## Notes

**2026-07-23T06:35:04Z**

Added --fast packaging mode that requires and reuses cli/target/release/replmux, skipping clean/build/tests while retaining linkage verification and packaging. Documented usage. Validated bash syntax, help, and unknown-argument failure without running a build.
