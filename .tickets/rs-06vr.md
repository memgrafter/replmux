---
id: rs-06vr
status: closed
deps: []
links: [mul-hfdn]
created: 2026-07-08T21:16:41Z
type: feature
priority: 1
assignee: memgrafter
tags: [jupyter-repl]
---
# Jupyter REPL manager: namespace kernels with CLI lifecycle + snapshot/restore

## Notes

**2026-07-08T21:17:32Z**

## Problem

Agents need persistent Python environments across turns. jupyter_repl.py provides the client but no lifecycle management — agents have to start kernels manually, track connection files, and can't namespace or snapshot state.

## Proposed solution

### CLI for kernel lifecycle
```
jupyter-repl create my-agent       # starts kernel, writes conn file under a name
jupyter-repl list                  # shows running kernels + names
jupyter-repl connect my-agent      # returns conn info for client code
jupyter-repl delete my-agent       # shuts down kernel
```

### Namespacing
Each kernel gets a human-readable name. Multiple agents can each have their own kernel, running simultaneously. Connection files stored in a predictable location (e.g. ~/.jupyter-repl/kernels/).

### Snapshot / restore (follow-up)
- Snapshot: serialize the kernel's Python namespace to disk
- Restore: start a fresh kernel and load the serialized state

### Skill
Teaches agents how to use both the CLI and the Python client (jupyter_repl.KernelClient).

## Scope

1. CLI for create/list/delete with namespacing
2. Skill for agent integration
3. Snapshot/restore as follow-up

**2026-07-23T05:23:11Z**

Core scope is complete in Rust: named create/list/connect/delete/exec lifecycle, extension integration, skill documentation, and persistent multiplayer namespaces. Snapshot/restore remains tracked separately by mul-hfdn.
