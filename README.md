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

Claude Code can load the `repl` and `repl-manage` tools directly:

```sh
claude mcp add --scope user replmux -- replmux mcp
```

- [Agent usage](SKILL.md)
- [CLI and installation](cli/README.md)
- [Agent-oriented kernel recommendations](docs/AGENT_KERNEL_CATALOG.md)
- [Blocked kernels and provider projections](docs/BLOCKED_KERNEL_DEPLOYMENT.md)
- [Kernel compatibility matrix](tests/jupyter-kernels/kernels.toml)
