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

Replmux is intentionally a runtime primitive, not a sandbox or durable database.
Kernel state disappears when its process dies, and executing code grants that
kernel the user's local permissions.

- [Agent usage](SKILL.md)
- [CLI and installation](cli/README.md)
- [Agent-oriented kernel recommendations](docs/AGENT_KERNEL_CATALOG.md)
- [Kernel compatibility matrix](tests/jupyter-kernels/kernels.toml)
