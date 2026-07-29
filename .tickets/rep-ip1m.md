---
id: rep-ip1m
status: open
deps: []
links: [mul-igbo]
created: 2026-07-29T22:58:29Z
type: bug
priority: 1
assignee: memgrafter
tags: [protocol, socket, reliability]
---
# Fix socket framing truncation and silent broker cap truncation

Socket request framing is broken for any payload that spans multiple TCP segments.

cli/assets/python_minimal_kernel.py:437-448 (_handle_socket_client) ends the read loop on `if len(chunk) < 65536: break`. That treats a short read as end-of-message, which is invalid on a stream socket: recv() returns bytes-available, not a complete frame. A payload split across segments exits after the first short read, leaving `data` holding a truncated JSON prefix. json.loads then fails with e.g. 'Unterminated string starting at: line 1 column 9 (char 8)' - char 8 is immediately after the opening {"code": " .

Trigger is segmentation, not size. Observed ~7KB succeeding and ~9KB failing, both far below any configured limit.

Second face of the same bug: the kernel raises, hits `finally: conn.close()`, and the Node client's sock.end(payload) in pi/extension/replTool.ts:104 then writes to a closed socket, surfacing as `write EPIPE`. The kernel survives with state intact and the cell never executes - a silent dropped execution.

Second, latent bug of the same class: cli/src/broker.rs:19 MAX_REQUEST_BYTES = 1MB is enforced at lines 251 and 315 via `.take(MAX_REQUEST_BYTES).read_to_end(...)`. `take` silently truncates rather than erroring, so a genuinely oversized request produces the same misleading JSON parse error instead of a clear 'request too large'.

All three clients half-close after writing (Rust: shutdown(Shutdown::Write) in broker.rs send_request and kernel.rs execute_socket; Node: sock.end()), so read-to-EOF is a safe fix and matches what the Rust broker already does.

## Design

Fix 1 (kernel, the real bug): drop the short-read heuristic and read until EOF. Clients already half-close, so EOF is well defined.

Fix 2 (broker, latent): read one byte past the cap and reject explicitly when the limit is exceeded, instead of silently truncating.

Non-goals: no length-prefix protocol change (would break wire compat with existing clients); no size-sweep test, which would find a fuzzy boundary and implicate the wrong cause.

## Acceptance Criteria

- A payload guaranteed to span multiple recv segments round-trips with byte-exact integrity.
- Oversized broker requests fail with an explicit size error, not a JSON parse error.
- No silent dropped executions: a failed request never leaves the kernel alive while reporting an ambiguous transport error.
- Existing single-segment execution behaviour unchanged.

## Notes

**2026-07-29T23:23:34Z**

Overlaps pre-existing mul-igbo (filed 2026-07-20, P4), which already identified the len(chunk) < 65536 heuristic as fragile. That ticket proposed length-prefix/NDJSON framing; this one fixes the truncation with read-to-EOF, the minimal change. Linked, and mul-igbo remains open for real framing.

STATUS: kernel framing fix verified behaviorally (5 stdlib unittest cases in cli/tests/test_socket_framing.py; mutation check confirms 3 of 5 fail against the old logic). Broker cap fix in cli/src/broker.rs is UNVERIFIED - Rust, not compiled, per the no-builds guidance. Needs cargo build/test before trusting.

Scope note: extracting recv_until_eof() into a module-level helper exceeded a strictly minimal fix; it was done to make the real code testable rather than testing a copy. Revert to an inline two-line fix if that tradeoff is unwanted.
