---
id: mul-04fh
status: closed
deps: []
links: []
created: 2026-07-23T03:54:02Z
type: task
priority: 1
assignee: memgrafter
---
# Add timed Rust dependency security audit script

## Notes

**2026-07-23T03:54:40Z**

Added executable scripts/audit-rust-deps.sh for cargo audit, cargo deny check, and cargo vet. It prints UTC start/end timestamps and elapsed seconds per check, runs all checks, aggregates failures, and reports missing prerequisites. Documented setup in cli/README.md. Syntax/diff checks pass; tools are not installed locally so audit run is pending.

**2026-07-23T03:55:50Z**

Audit script now checks each cargo subcommand first and automatically installs only missing cargo-audit, cargo-deny, or cargo-vet with --locked. Existing tools are reused; installations are also timestamped/timed. Documentation updated.

**2026-07-23T04:05:21Z**

Reviewed first audit run: cargo audit passed with 189 dependencies and 0 known RustSec vulnerabilities. cargo deny had no deny.toml, so default policy rejected all licenses (173 reports); advisories/bans/sources passed, with duplicate warnings for bitflags/getrandom/syn/windows-sys. cargo vet failed only because supply-chain config is uninitialized. Tool installs succeeded and will be reused.

**2026-07-23T04:08:02Z**

Added committed cli/deny.toml with explicit permissive-license allowlist, crates.io-only sources, denied wildcards, and duplicate warnings. Initialized and committed cargo-vet supply-chain baseline (config/audits/imports). Script now bootstraps missing deny policy and runs cargo vet init --locked when needed. Validated configured run: audit passed, deny advisories/bans/licenses/sources passed with 4 duplicate warnings, vet passed; total 0 failures.
