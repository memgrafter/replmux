---
id: rep-xnyr
status: open
open: true
deps: []
links: []
created: 2026-09-13T17:19:46Z
type: feature
priority: 2
assignee: memgrafter
---
# Expose read-only kernel availability for multiplayer workspaces

Let collaborating agents inspect whether a named persistent workspace is busy or available without submitting code or waiting behind an execution lock. Heartbeat currently reports liveness, not availability. The minimal worker serializes execution with a lock, so concurrent calls can implicitly wait, but there is no explicit observable application-level execution queue or admission contract.

Scope: add a bounded, read-only status path reachable while execution is busy; distinguish idle, running, dead, and unknown/unsupported rather than inferring idle from heartbeat. Report active execution identity and elapsed time where available, plus last completion/error state. Surface availability through CLI/broker and Pi management without exposing code or namespace contents unnecessarily. Define pending-work semantics per transport; do not invent queue depth or FIFO guarantees from lock contention.

Multiplayer caveat: an idle snapshot is advisory, not a reservation. Document the check-then-execute race and identify whether a separate atomic try-execute/busy-rejection or lease mechanism is needed. A full scheduler, fairness policy, and queue implementation are not required by this ticket.

Acceptance: two clients share one kernel; one runs bounded work while the other obtains prompt status without mutating state or queuing a Python probe. Status remains busy after the first client disconnects while its computation continues, then transitions after completion/error. Cover competing submissions, persistent state, failure recovery, and dead kernels. Clearly distinguish unknown queue state from zero pending work.

Evidence: after Pi Esc aborted the minimal-worker wait, heartbeat remained alive and macOS thread sampling still showed builtin_exec -> time_sleep -> nanosleep. Related interruption work: rep-z9nu and rep-53d7. Ticket only; no implementation requested.
