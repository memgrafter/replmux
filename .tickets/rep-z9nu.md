---
id: rep-z9nu
status: in_progress
open: true
deps: []
links: [rep-53d7, rep-kmvq]
created: 2026-09-13T15:35:32Z
type: bug
priority: 1
assignee: memgrafter
---
# Propagate Pi abort to Replmux transports and kernel interruption

Pi REPL socket paths ignore AbortSignal, leaving Esc waiting for tool completion. Add prompt local request cancellation and independent bounded kernel interrupt delivery where supported; do not equate disconnect with cancellation. Cover abort races, fallback, and retained-state behavior; link rep-53d7.

## Notes

**2026-09-13T15:42:03Z**

Implemented (not committed): extracted abort-aware socket/CLI transport helper; socket abort/close/timeout cleanup; no execution replay after dispatch; independent one-second interrupt delivery through the selected broker or local CLI; explicit minimal-worker unsupported and cancellation-unconfirmed errors; CLI waiting is abortable independently of process exit. Pi management now supports create kernelspec and uses canonical kernel subcommands. Added pi/README.md and socket/CLI regression fixtures including cross-call fixture state and failure recovery. Node syntax checks passed for all three TypeScript files; git diff --check passed. Automated fixtures and live Pi/kernel E2E were NOT run under the earlier test restriction. No builds, installs, config edits, broker restarts, kernel launches, or commits. Global stale settings entry remains at ~/.pi/agent/settings.json:75; user was informed. Keep in progress pending authorized tests. Kernel interruption remains best-effort/name-scoped, not per-execution cancellation.
