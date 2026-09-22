# Replmux

Agents often repeat expensive setup because each tool call starts a fresh
process. Replmux keeps a named Jupyter kernel alive, so variables, imports, and
results survive across turns and can be shared intentionally between agents.

That makes computation a workspace rather than a disposable command:

```sh
replmux kernel create analysis
replmux kernel exec analysis 'values = [3, 5, 8]'
replmux kernel exec analysis 'sum(values)'
# 16
```

## Claude Code

One command gives every Claude Code session on the machine two tools, `repl` and
`repl-manage`, backed by the same kernels as the command line:

```sh
claude mcp add --scope user replmux -- replmux mcp   # once; then start a new session
claude mcp list                                      # should show "replmux ... Connected"
```

In a session, ask Claude to create a workspace (`repl-manage` with action `create`), run code in
it (`repl` with the workspace name and the code), and delete it when the task is done. State
stays between calls, so data is loaded once and reused. Three things to know: a value is shown
only when a call is a single expression, otherwise print it; returned values are never
truncated, so keep them small; and a call that never ends (an endless loop) cannot be
interrupted in the default worker and blocks every other call until that workspace is deleted.
A full test record with these findings is in `.tickets/rep-icbz.md`.

Replmux speaks the standard Jupyter protocol. The same lifecycle works with
installed kernels for Julia, R, C++, JavaScript, .NET, and domain systems such
as SageMath—not only Python. Its bundled minimal Python worker adds a fast local
socket for agent tools, while standard kernels use signed ZeroMQ channels.

Why multiple kernels? Agents can use the system that expresses the problem
most directly: Python for general work, Julia or Fortran for numerical code, R
for statistics, C++/Rust/C# for typed systems work, JavaScript for web data,
SageMath for exact mathematics, and SQLite for stateful relational analysis.
Elixir (IElixir) and C++ ROOT (JupyROOT) have also passed manual persistent-state
validation in pinned Docker environments, with important limitations.
The sixteen cataloged implementations—fourteen executable-matrix entries plus
these two manual validations—and concrete use cases are listed in
[`SKILL.md`](SKILL.md#ready-kernels-and-use-cases). See the
[validation report](docs/WIKI_TOP_KERNEL_VALIDATION.md) for ROOT's false-success
error reporting and the interruption caveats, including xeus-cpp state loss on
SIGINT. Validation does not imply that every protocol feature passes.

Replmux is intentionally a runtime primitive, not a sandbox or durable database.
Kernel state disappears when its process dies, and executing code grants that
kernel the user's local permissions.

- [Agent usage](SKILL.md)
- [CLI and installation](cli/README.md)
- [Agent-oriented kernel recommendations](docs/AGENT_KERNEL_CATALOG.md)
- [Blocked kernels and provider projections](docs/BLOCKED_KERNEL_DEPLOYMENT.md)
- [Kernel compatibility matrix](tests/jupyter-kernels/kernels.toml)
