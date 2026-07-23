---
id: mul-2bti
status: in_progress
deps: []
links: []
created: 2026-07-23T05:24:45Z
type: feature
priority: 1
assignee: memgrafter
---
# Automate macOS and GNU Linux releases

## Notes

**2026-07-23T05:26:17Z**

Added native GitHub Actions matrix for aarch64/x86_64 macOS and GNU Linux. Each runner verifies its host target, prepares locked Python environments, runs full release build/tests, verifies static libzmq via release.sh, checks archive/checksum, and uploads per-target artifacts. Added manual and v* tag triggers plus documentation. YAML/shell/diff validation passed; first CI matrix run pending.
