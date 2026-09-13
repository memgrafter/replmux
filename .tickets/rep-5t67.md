---
id: rep-5t67
status: closed
open: false
deps: []
links: [rep-53d7]
created: 2026-09-13T12:42:03Z
type: task
priority: 2
assignee: memgrafter
---
# Validate wiki-top IElixir and C++ kernels

Identify IElixir, ROOT C++, and xeus-cpp from the Jupyter wiki; validate installed runtimes using the existing Replmux binary, persistent state, error recovery, and lifecycle checks. Record blockers without running builds.

## Notes

**2026-09-13T12:52:36Z**

Validated all three wiki-top entries with existing binaries: xeus-cpp 0.10.0, IElixir prebuilt Elixir 1.11.2/OTP23, ROOT 6.38.00. Persistent state and recovery work; busy interruption failed for all three. ROOT falsely reports invalid code as ok:true and cold startup exceeded 5s. Recorded 85 command responses, environment workarounds, and pinned images in docs/WIKI_TOP_KERNEL_VALIDATION.md and tests/jupyter-kernels/results/wiki-top-2026-09-13.json. Corrected xeus-cpp matrix display/project URL, preserving legacy ID. JSON/TOML assertions, installer dry-run, and diff check passed. No builds. Follow-up interrupt ticket rep-53d7.
