---
id: mul-igbo
status: open
deps: []
links: []
created: 2026-07-20T05:21:38Z
type: task
priority: 4
assignee: memgrafter
---
# Robust framing for kernel Unix socket protocol

_handle_socket_client uses len(chunk) < 65536 as a heuristic to detect end-of-data. This works because the extension calls sock.end() but is fragile for large messages or network buffers. Should use length-prefix or newline-delimited (NDJSON) framing for robustness.
