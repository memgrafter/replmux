# Multirepl Rust CLI

A small HTTP client for the runtime CRUD operations exposed by `service/`.

## Commands

```bash
multirepl runtime create analysis
multirepl runtime list
multirepl runtime get rt_ID
multirepl runtime update rt_ID --status running
multirepl runtime delete rt_ID
```

The default API URL is `http://127.0.0.1:8000`. Override it with either:

```bash
multirepl --api-url http://server:8000 runtime list
MULTIREPL_API_URL=http://server:8000 multirepl runtime list
```

Use `--json` for machine-readable output:

```bash
multirepl --json runtime list
```

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
