# Multirepl Rust CLI

The CLI manages runtime metadata through `service/` and provides local kernel lifecycle and persistent REPL execution.

## Runtime commands

```bash
multirepl runtime create analysis
multirepl runtime list
multirepl runtime get rt_ID
multirepl runtime update rt_ID --status running
multirepl runtime delete rt_ID
```

## Kernel and REPL commands

```bash
multirepl kernel create analysis
multirepl kernel list
multirepl kernel connect analysis
multirepl kernel exec analysis 'x = 42'
multirepl kernel exec analysis 'x'
multirepl kernel delete analysis
```

For compatibility with `jupyter_repl_cli.py`, lifecycle commands are also accepted at the top level:

```bash
multirepl create analysis
multirepl list
multirepl connect analysis
multirepl exec analysis 'x + 1'
multirepl delete analysis
```

Kernel configuration can be supplied globally or through environment variables:

```text
--kernel-dir       MULTIREPL_KERNEL_DIR
--python           MULTIREPL_PYTHON
--kernel-script    MULTIREPL_KERNEL_SCRIPT
--broker-socket    MULTIREPL_BROKER_SOCKET
```

The defaults are `~/.jupyter-repl/kernels`, `python3`, `minimal_kernel_clean.py`, and the short broker path `~/.multirepl/b.sock`. Use `--json` for stable machine-readable lifecycle and execution responses.

### Local and served modes

Kernel commands default to `--transport auto`. Each command attempts the broker Unix socket once. An active broker receives the real request; `ENOENT` or `ECONNREFUSED` immediately short-circuits to the same in-process service implementation. Permission, protocol, and other broker failures are returned rather than bypassed.

```bash
multirepl kernel list                    # auto: socket or in-process
multirepl --transport local kernel list  # require in-process handling
multirepl --transport socket kernel list # require a running broker
multirepl serve                          # explicit persistent broker
```

The broker socket is created with mode `0600`, stale sockets are replaced on startup, and requests use bounded 30-second I/O timeouts.

The default API URL is `http://127.0.0.1:8000`. Override it with either:

```bash
multirepl --api-url http://server:8000 runtime list
MULTIREPL_API_URL=http://server:8000 multirepl runtime list
```

Use `--json` for machine-readable output:

```bash
multirepl --json runtime list
```

## Release package

From the repository root:

```bash
./scripts/release.sh
```

The script cleans previous Rust build artifacts, runs the locked service and CLI test suite, builds the optimized binary, verifies that libzmq is statically bundled rather than dynamically linked, and creates a target-specific archive plus SHA-256 checksum under `dist/`. Override the destination with `MULTIREPL_RELEASE_DIR`.

The archive includes the CLI, `minimal_kernel_clean.py`, and this README. The Rust CLI uses its bundled libzmq for Jupyter control messages and does not require a system ZeroMQ installation. Python 3 with `pyzmq` is still required by the Python kernel worker.

## Dependency security

Run all checks with UTC start/finish timestamps and elapsed seconds:

```bash
./scripts/audit-rust-deps.sh
```

The script checks for `cargo-audit`, `cargo-deny`, and `cargo-vet`, installing only missing tools with `cargo install --locked`. Existing installations are reused. It initializes missing cargo-deny and cargo-vet policy stores, runs every security check even if an earlier check fails, then exits nonzero if any check failed.

Security policy is committed in `cli/deny.toml` and `cli/supply-chain/`. The initial cargo-vet exemptions establish a reproducible baseline; they remain review debt to replace with imported or project audits over time.

## Development

Format and inspect metadata without compiling:

```bash
cargo fmt --check
cargo metadata --no-deps
```

When builds are permitted, run:

```bash
cargo test
```
