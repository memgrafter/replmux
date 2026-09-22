---
id: rep-icbz
status: closed
open: false
deps: []
links: [rep-1gph, rep-xdgn, rep-2aad]
created: 2026-09-18T17:41:19Z
type: report
priority: 1
assignee: memgrafter
tags: [claude-code, mcp, battery, reference]
---
# Claude Code MCP battery: repl / repl-manage results and rules (2026-09-18)

Battery run 2026-09-18 from Claude Code (replmux 0.1.0 registered with `claude mcp add --scope user replmux -- replmux mcp`; tools `repl` and `repl-manage`; default minimal worker on Homebrew Python 3.14.6, arm64). Every item below was exercised through the real MCP tools in a live session, after a first pass that drove `replmux mcp` over stdio by hand with the same results.

- create named: ok. create with no name: ok, auto-named `repl-<pid>-<time>`. create duplicate name: refused, "already running (pid N)".
- run on unknown kernel: "kernel 'x' not found". delete unknown: same error. delete then run: same error.
- statement: `(ok)`. single expression: value shown. statements followed by an expression: `(ok)` only, value hidden. put the expression in its own call or print it.
- stdout and stderr: both captured, labeled `stdout:` / `stderr:`.
- exception: type and message only, no traceback. syntax error: reported with line. kernel state survives both.
- working directory: the session's cwd (repo root); `sys.executable` is the Homebrew python.
- shell-hostile string (quotes, `$HOME`, backticks, parens, angle brackets, backslash): byte for byte intact. unicode incl. CJK and emoji: intact.
- large value: a 20,000-char string returned in full. no truncation at all: slice or summarize before returning big values or the context floods.
- isolation: a name set in one kernel is a NameError in another.
- two calls to one kernel issued in the same turn: ran in order.
- connect: returns the connection record, including the socket path and the HMAC key.
- libraries visible: numpy, zmq, json, sqlite3 yes; pandas, requests no.
- long call: a 40 s sleep completed, no timeout. Claude Code moves any tool call still running at 120 s to a background task and reports later; the server's own execution timeout fired at about 5 min with "timed out waiting for REPL execution".
- runaway (`while True`): the kernel sits at 100% CPU. `replmux kernel interrupt` on the default worker fails: "the minimal worker does not support non-destructive interruption; use a standard kernelspec or explicitly delete the kernel". TaskStop on the Claude side ends the wait but not the loop.
- serialization: while one kernel is stuck, calls to every other kernel through the MCP server block too. Confirmed twice: a call sent at 10:40:16 ran at 10:40:46, the second the stuck kernel was deleted. The only recovery is `repl-manage delete` (or `replmux kernel delete`), which loses that kernel's state.
- no Jupyter kernelspec is installed on this machine (`jupyter` absent, no ipykernel), so the interruptible path was not testable here.
- cleanup: all battery kernels deleted; four long-running kernels from other agents left untouched.

Rules that follow: one expression per call to see a value; never run unbounded loops in the default worker; keep returned values small; if a kernel hangs, delete it and expect the others to unblock.

## Notes

**2026-09-18T17:41:58Z**

Docs: README.md gained a 'Claude Code' section right after the first example (setup command, health check, how the two tools are used, the three cautions); AGENTS.md gained a 'Claude Code, cut to the chase' list near the top. Both point here for the full record.
