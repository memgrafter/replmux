# Agent guide

Replmux gives agents named, persistent Jupyter workspaces. Preserve state across
calls; use separate names for isolation and shared names for deliberate
collaboration.

- Read [`SKILL.md`](SKILL.md) for usage.
- Rust CLI and protocol client: `cli/`
- Minimal Python worker: `cli/assets/python_minimal_kernel.py`
- Pi tools: `pi/extension/`
- Kernel matrix: `tests/jupyter-kernels/`
- Create a `tk` ticket before changing code.
- Do not run builds unless the user explicitly asks; keep changes focused and
  leave unrelated worktree changes alone.
- Test persistent state and failures, not only one-shot execution.
- Treat every kernel as unsandboxed arbitrary code.

## First-time setup

1. **Build the binary** — `scripts/release.sh` produces a `dist/` archive containing
   the `replmux` binary (or `cargo build --release` in `cli/`).
2. **Put `replmux` on PATH** — e.g. `~/.local/bin/`.
3. **Provision the runtime** — `scripts/setup-runtime.sh` (after the binary exists):
   creates a dedicated `~/.venvs/replmux` with `pyzmq` for the minimal kernel
   worker, stages the kernel script, and installs the broker as a user systemd
   service. On a host where `pip`/`ensurepip` are unavailable it relocates a
   manylinux pyzmq wheel from the club-3090 venv (see the script header). The Rust
   binary bundles its own libzmq.

## Build & CI scripts (`scripts/`)

- `release.sh` — release packaging: cleans `target/`, runs the locked service + CLI test suites, builds the optimized binary, verifies libzmq is statically bundled, and writes the archive + SHA-256 to `dist/` (override with `REPLMUX_RELEASE_DIR`). `--static`/`--arch` cross-build a fully-static musl Linux binary; `--fast` packages an existing binary without rebuilding. GitHub Actions (`release.yml`) runs this on tags.
- `build-and-test.sh` — CI-style check: host release build + `cargo test`, or with `--target <triple>` a cross build, test compilation, and static-binary verification; `--vm <name>` pushes the binary into a running sandmux VM for a live smoke test.
- `lib-cross.sh` — shared cross-compilation support (sourced by the two scripts above): clang/lld plus a downloaded Alpine musl sysroot for `*-unknown-linux-musl` targets.
- `audit-rust-deps.sh` — supply-chain audit: installs missing `cargo-audit`/`cargo-deny`/`cargo-vet` once, writes their policy files, and runs all checks even if one fails.
- `setup-runtime.sh` — runtime provisioning, run after the binary exists: creates a dedicated `~/.venvs/replmux` with `pyzmq`, copies the minimal kernel worker to `~/.local/share/replmux/`, and installs/enables the `replmux` broker as a user systemd service (`replmux.service`, restart-on-failure). Idempotent; installs nothing else.

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
