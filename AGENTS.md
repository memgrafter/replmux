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

Deeper operational details live in [`cli/README.md`](cli/README.md); kernel
recommendations live in
[`docs/AGENT_KERNEL_CATALOG.md`](docs/AGENT_KERNEL_CATALOG.md).
