---
id: rep-x3ek
status: open
deps: []
links: []
created: 2026-07-23T19:01:39Z
type: task
priority: 2
assignee: memgrafter
---
# Validate MCP interface with Claude CLI, Claude App, Codex CLI, Codex App


Test `replmux mcp` stdio server against all four client targets:
- Claude CLI (stdio MCP)
- Claude App (GUI, developer + non-developer modes)
- Codex CLI (code mode — can we swap in our tools or must we deregister theirs?)
- Codex App (GUI)

Questions to answer:
- Can we register repl tools alongside the client's native tools, or do they conflict?
- Does code mode in Codex allow tool substitution or require deregistration?
- Any protocol version mismatches (MCP 2024-11-05)?
- Tool call formatting differences between clients?

