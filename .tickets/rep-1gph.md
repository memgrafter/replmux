---
id: rep-1gph
status: open
open: true
deps: []
links: [rep-icbz, rep-ip1m]
created: 2026-09-18T17:46:16Z
type: bug
priority: 1
assignee: memgrafter
tags: [claude-code, mcp, battery]
---
# MCP tool returns unbounded output; cap and mark truncation

Found in the Claude Code battery (rep-icbz). A `repl` call returning a 20,000-character string came back whole; nothing caps the size of a value, stdout, or stderr. Through an MCP client every byte lands in the model's context, so one careless call (a big list, a dataframe repr, a file read) floods the conversation.

Proposal: cap the text the MCP tool returns, default around 4-8k characters, with a clear marker such as `... [truncated, 20000 chars total]` and the total size. Keep the CLI uncapped, or give it the same cap behind a flag. Optional: a `max_chars` argument on `repl`. Test: the battery's `"x" * 20000` case should return a bounded string plus the marker.

## Notes

**2026-09-18T17:48:04Z**

PROPOSED FIX (after reading cli/src/mcp.rs and cli/assets/python_minimal_kernel.py).

Where the size comes from: `McpServer::call_repl` (mcp.rs ~L96-125) concatenates `response.result`, `stdout` and `stderr` verbatim; the worker's `_execute_direct` puts `repr(value)` and the full captured streams in the JSON. Nothing measures either side.

Fix, in one place with one visible rule:
1. Add `fn clip(text: &str, max: usize) -> String` in mcp.rs: keep the head, append `\n… [clipped: N chars total, showing M]`. Apply it separately to the value, stdout and stderr in `call_repl` so a chatty stdout cannot hide the value, then once more to the combined text as a hard ceiling. Defaults: 4,000 chars per part, 8,000 total.
2. Make the ceiling settable: `replmux mcp --max-output-chars N` (McpServer field, like `broker_socket`), and an optional `max_chars` integer in the `repl` tool schema (`tool_definitions`, mcp.rs ~L178) for a one-off larger read. Clamp to a sane maximum (e.g. 64k) so a model cannot ask for a megabyte.
3. Do not clip in the worker or the CLI: `replmux kernel exec` keeps returning everything (scripts depend on it), and the worker keeps returning the full payload so the CLI path is unchanged. The clip is a property of the MCP tool only.
4. Reuse the same `clip` for the broker path that rep-ip1m says silently truncates, so both paths mark truncation the same way.

Tests: `"x" * 20000` through the tool returns <= 8,000 chars ending with the marker and the true total; `print("y" * 20000); 1` shows the value `1` first and a clipped stdout; `max_chars: 20000` returns it whole; the CLI still returns 20,000 chars. Unit tests live next to the existing ones in mcp.rs (they already build a `McpServer` without a kernel).
