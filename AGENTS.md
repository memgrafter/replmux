# Agent guide

Replmux gives agents named, persistent Jupyter workspaces. Preserve state across
calls; use separate names for isolation and shared names for deliberate
collaboration.

- Read [`SKILL.md`](SKILL.md) for usage.
- Rust CLI and protocol client: `cli/`
- Minimal Python worker: `cli/assets/minimal_kernel_clean.py`
- Pi tools: `pi/extension/`
- Kernel matrix: `tests/jupyter-kernels/`
- Create a `tk` ticket before changing code.
- Do not run builds unless the user explicitly asks; keep changes focused and
  leave unrelated worktree changes alone.
- Test persistent state and failures, not only one-shot execution.
- Treat every kernel as unsandboxed arbitrary code.

## Ready kernel choices

Choose the narrowest capable kernel instead of assuming Python:

1. Python — automation, data, libraries, and general analysis.
2. Julia — numerical science, optimization, and high-performance arrays.
3. R — statistics, models, and statistical graphics.
4. C++ — native APIs, compiler behavior, and performance prototypes.
5. JavaScript — JSON, web logic, Node APIs, and async experiments.
6. C# — .NET APIs, LINQ, and typed application logic.
7. SageMath — exact symbolic algebra, number theory, and combinatorics.
8. LFortran — modern or legacy Fortran and numerical routines.
9. xeus-sqlite — stateful SQL, schemas, joins, and query plans.
10. EvCxR — Rust ownership, type, compiler, and systems experiments.
11. xeus-lua — lightweight embedded scripting and Lua semantics.
12. xeus-r — native-protocol R compatibility and statistical cross-checks.
13. xeus-python — native-protocol Python compatibility and alternate behavior.
14. xeus-sql — stateful SQL across SQLite and configured database backends.

These are lifecycle-tested entries in `tests/jupyter-kernels/kernels.toml`.
Discover the installed kernelspec name before launch; names can vary by version.

Deeper operational details live in [`cli/README.md`](cli/README.md); kernel
recommendations live in
[`docs/AGENT_KERNEL_CATALOG.md`](docs/AGENT_KERNEL_CATALOG.md). For a failed
kernel, consult
[`docs/BLOCKED_KERNEL_DEPLOYMENT.md`](docs/BLOCKED_KERNEL_DEPLOYMENT.md) before
changing packages or moving providers.
