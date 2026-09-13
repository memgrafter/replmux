# Pi extension

Load the single-file `pi/extension/replTool.ts` through Pi's extension
configuration, for example in `~/.pi/agent/settings.json`:

```json
{
  "extensions": ["~/code/replmux/pi/extension/replTool.ts"]
}
```

Preserve other configured extensions. Replace any old
`~/code/prim/prototype/pi-extension/replTool.ts` entry after relocating the repo.
Reload Pi's extensions with `/reload` after updating the configuration/code.
Copy or symlink only `replTool.ts`; transport code is inline, with no relative
module dependencies. Pi provides the extension's package imports.

## Interruptible Python

The default `repl-manage` create action still launches the minimal Python worker.
It does **not** support non-destructive cancellation. To create an interruptible
Python workspace, discover an installed ipykernel spec, then use:

```text
repl-manage { action: "create", name: "interruptible", kernelspec: "python3" }
repl { name: "interruptible", code: "answer = 42" }
repl { name: "interruptible", code: "import time; time.sleep(60)" }
```

A `kernel.json` path is also accepted for `kernelspec`. Availability and paths
are host-specific; `python3` is an example, not an automatically provisioned spec.
This does not convert an existing minimal workspace or preserve its state.

## Esc and steering

- Esc aborts the current Pi tool wait. Socket transports disconnect promptly;
  CLI execution also stops waiting and forwards Pi's signal to its client process.
- If execution may have been submitted to a standard kernel, the extension sends
  a **separate** interrupt request through the same broker, or a local CLI control
  request for the CLI execution path. Control delivery has a one-second deadline
  independent of the already-aborted execution signal. Pi can therefore take up
  to roughly one second to finish reporting an abort (excluding event-loop delays).
- Minimal-worker aborts report unsupported cancellation: code may keep running.
  The extension never deletes/restarts a kernel or escalates to termination.
- SIGINT delivery and message acknowledgment do **not** confirm cancellation or
  state preservation. Control timeouts/errors also leave execution uncertain;
  even a timed-out request may already have been delivered. Verify the runtime
  before reusing it. Some kernels, including the tested xeus-cpp build, exit on
  SIGINT.
- An already-aborted call submits nothing. Execution is not retried after dispatch,
  and a completed request no longer reacts to later aborts.
- Steering messages wait until the current tool batch finishes; they are not
  kernel interrupt commands.

Interruption targets a named kernel, not an individual execution request. Do not
share a kernel concurrently when relying on cancellation. Submission/start/finish
can race with control delivery; this extension does not provide request-scoped
cancellation or prove that queued code cannot subsequently execute.

The extension defaults to `~/.local/bin/replmux` (`REPLMUX_BINARY` overrides it)
and `~/.replmux/b.sock` (`REPLMUX_BROKER_SOCKET` overrides it). A create-only
`binary` parameter does not change the binary used by later `repl` calls.
Use a mode-aware binary, restart its broker, and recreate standard kernels to
obtain verified signal interruption metadata. Reloading the extension alone does
not update an installed binary or running broker. See
[`cli/README.md`](../cli/README.md#interruption).

## Regression checks

On Node with native TypeScript stripping (Node 24+):

```bash
node --test pi/tests/transport.test.ts
```

The tests load the marked inline transport section using Node's TypeScript
stripping API, without requiring Pi's UI packages or a separate transport file.
They also guard against relative module dependencies in the extension.
These are local socket/CLI fixtures, not real-kernel or Pi keyboard E2E tests.
They cover abort timing, independent control delivery, failure/timeout reporting,
listener cleanup, fallback, repeated abort, and cross-call fixture state. Actual
ipykernel state retention and Pi's Esc behavior require a separate manual E2E run.
