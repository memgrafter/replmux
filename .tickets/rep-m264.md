---
id: rep-m264
status: in_progress
deps: []
links: []
created: 2026-07-23T18:33:31Z
type: task
priority: 2
assignee: memgrafter
---
# Create Sage uv REPL kernel

## Notes

**2026-07-22T22:53:36Z**

Blocked: sagemath-standard 10.7 resolves under uv/Python 3.12, but no usable macOS ARM wheels exist (including prereleases). Installing would require a prohibited source build. Removed incomplete .venv-sage.

**2026-07-22T22:55:41Z**

Proceeding with prebuilt conda-forge Sage, then uv system-site-packages wrapper and repl-manage kernel.

**2026-07-22T23:05:25Z**

Installed prebuilt SageMath 10.9 at ~/.local/share/sage-env, created uv wrapper ~/.local/share/uv-venvs/sage-repl, launched repl kernel ana-20260722230349, and independently verified det DF=-2, all three collisions, and the elimination cubic with native Sage/Singular.
