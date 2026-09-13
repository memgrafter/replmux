---
id: rep-53d7
status: in_progress
open: true
deps: []
links: [rep-5t67, rep-z9nu]
created: 2026-09-13T12:49:52Z
type: bug
priority: 1
assignee: memgrafter
---
# Honor kernelspec interrupt mode and verify cancellation during execution

Wiki-top validation found that Replmux control interrupt requests did not cancel bounded 20-second work in xeus-cpp 0.10.0, IElixir (Elixir 1.11.2), or ROOT 6.38.00. Specs default to signal mode, but the current source always sends interrupt_request. Add mode-aware cancellation and test state recovery while busy, not only idle acknowledgments. See docs/WIKI_TOP_KERNEL_VALIDATION.md and rep-5t67.

## Notes

**2026-09-13T13:46:15Z**

## Scope and implementation requirements

- Preserve kernelspec interrupt_mode when creating a managed kernel. Honor the Jupyter default of signal when omitted; route message mode to the control-channel interrupt_request.
- For signal mode, use verified guest/local process ownership and identity. Define launcher/process-group handling explicitly; do not blindly signal an arbitrary PID from an attached connection file or stale metadata.
- Keep interruption reachable while execution is busy across supported direct and broker paths. A long execution must not hold a lock or monopolize a request path needed for interruption.
- Separate request delivery/acknowledgment from confirmed execution cancellation. An idle interrupt_reply is not proof of cancellation.
- Define bounded outcomes for unsupported interruption, missing process ownership, timeout, and kernel exit. Do not silently turn a state-preserving interrupt into destructive kernel termination. Deleting/restarting a kernel is a separate operation with explicit state-loss semantics.
- Document that a client-side execution timeout or disconnected client does not itself cancel code still running in the kernel.

## Acceptance criteria

1. Deterministic fixtures cover omitted/default signal mode, explicit signal mode, and explicit message mode, including malformed mode handling.
2. Start bounded long-running code, confirm it is executing using a marker or execution event, then interrupt through a separate request. Verify completion before its natural deadline, not merely a successful interrupt reply.
3. For kernels that support non-destructive cancellation, read previously assigned state and successfully execute new code after interruption. Record unsupported behavior or kernel death explicitly rather than marking it as a pass.
4. Cover idle interruption, already-exited kernels, stale PID metadata, attached kernels without managed process ownership, and repeated interrupts. No unrelated process may be signaled.
5. Cover concurrent execution/control through supported direct and broker transports, with bounded test deadlines and cleanup of all test-owned processes/artifacts.
6. Rerun the recorded xeus-cpp 0.10.0, IElixir, and ROOT probes when their runtimes are available, reporting provider/version-specific limitations separately. Native SIGINT was NOT tested in the original validation; do not assume it is unsupported.
7. Update the compatibility report and agent-facing interrupt documentation to distinguish acknowledgment, actual cancellation, and destructive termination.

## Evidence and boundaries

Current source: cli/src/kernelspec.rs (kernelspec metadata), cli/src/kernel.rs KernelManager interrupt path, and cli/src/jupyter.rs JupyterClient::interrupt, which currently always sends a control message. Recorded evidence: docs/WIKI_TOP_KERNEL_VALIDATION.md and tests/jupyter-kernels/results/wiki-top-2026-09-13.json.

All three busy interrupt probes exceeded a five-second external deadline; the original twenty-second computations completed naturally and their prior state remained readable. xeus-cpp acknowledged an idle request; IElixir also timed out while idle.

This ticket covers Replmux only. Sandmux shell-child timeout handling and its JSON timeout exit-code issue are separate concerns and are not included. Ticket refinement only; no implementation or new tests performed.

**2026-09-13T13:56:33Z**

Minimal implementation added, not committed: parse/default kernelspec interrupt_mode; isolate managed standard kernels in a process group; save mode, PID, and OS start identity in a private .interrupt sidecar; verify launch identity before group SIGINT on Linux/macOS; preserve control-message replies for message-mode and legacy/attached kernels. Broker/CLI now distinguish signal_sent from a Jupyter reply and explicitly mark cancellation unconfirmed. Custom minimal-worker interruption returns unsupported rather than a misleading acknowledgment. No automatic termination escalation. Documented recreate/restart requirements and detached-launcher limitations. Host macOS release build passed using cargo build --manifest-path cli/Cargo.toml --locked --release --bin replmux; git diff --check passed. No tests or kernel launches performed at user request; no installed binary replacement or broker restart. Linux build, runtime cancellation/state recovery, and full ticket acceptance remain pending. Process identity checks reject stale metadata but are not an atomic OS process-handle guarantee across the final check/signal race.

**2026-09-13T14:04:30Z**

Manual E2E run with the already-built macOS release binary (SHA256 f9371051d8c22bf03807d0566061086cc3b9425ecb327fa066c8a9a570806533), no rebuild or source edits. Used temporary kernelspec copies with relocated paths corrected and an isolated broker/runtime directory. ipykernel 7.3.0 / Python 3.14.6: default signal mode passed direct (31ms) and broker (37ms); explicit message mode passed broker (40ms). Each started a marker-confirmed 20s sleep, interrupted through another built-binary request, returned KeyboardInterrupt before natural completion, retained answer=42, and passed heartbeat. Timings are single manual observations, not benchmarks. xeus-cpp 0.10.0 default signal mode: SIGINT delivery returned in 4ms, but the process exited, heartbeat failed, and a subsequent state read timed out waiting for kernel_info_request. The original execution client stayed pending and required manual termination; this is NOT a non-destructive cancellation pass. A repeat interrupt refused the dead process identity. Fresh xeus-cpp create/info/delete still worked afterward. All test kernels and private broker were stopped, and runtime artifacts were removed. Evidence and full commands: /tmp/rmx-int-kvzahbeu/results.json (local temporary transcript). Keep ticket open: xeus-cpp state-preserving interruption and prompt execution-client failure on kernel death remain unresolved; IElixir/ROOT and Linux were not retested.
