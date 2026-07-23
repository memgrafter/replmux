---
id: mul-hmnm
status: closed
deps: []
links: []
created: 2026-07-23T03:46:40Z
type: task
priority: 1
assignee: memgrafter
---
# Add Rust CLI release script

## Notes

**2026-07-23T03:47:42Z**

Added executable scripts/release.sh. It runs locked build/tests, creates a target/version-specific tar.gz containing the CLI, kernel worker, and README, writes SHA-256 checksum, supports MULTIREPL_RELEASE_DIR, and documents target pyzmq requirement. bash syntax and git diff checks passed; release command not run per no-build instruction.

**2026-07-23T03:48:46Z**

Release run exited 141 because pipefail propagated rustc SIGPIPE when host_target awk exited before consuming all rustc -vV output. Removed the early awk exit so the pipeline drains normally. Syntax and diff checks pass; awaiting rerun.

**2026-07-23T03:49:30Z**

Updated release flow to run cargo clean against cli/Cargo.toml before locked build and tests; documented clean release behavior. Shell syntax and diff checks pass.
