---
id: mul-igbo
status: open
deps: []
links: [rep-ip1m]
created: 2026-07-20T05:21:38Z
type: task
priority: 4
assignee: memgrafter
---
# Robust framing for kernel Unix socket protocol

_handle_socket_client uses len(chunk) < 65536 as a heuristic to detect end-of-data. This works because the extension calls sock.end() but is fragile for large messages or network buffers. Should use length-prefix or newline-delimited (NDJSON) framing for robustness.

## Notes

**2026-07-29T23:23:34Z**

Root cause confirmed and the truncation half is fixed under rep-ip1m: the len(chunk) < 65536 heuristic was not merely fragile, it was actively truncating multi-segment payloads in practice. Reproduced against real Unix sockets: a 9,018-byte request split 4000/5018 was truncated to 4000 bytes, producing 'Unterminated string starting at: line 1 column 10' plus EPIPE on the client. _handle_socket_client now reads to EOF via a recv_until_eof() helper, which all current clients support since each half-closes after writing.

This ticket's original ask (length-prefix or NDJSON framing) is still OPEN and not addressed. Read-to-EOF fixes correctness for the existing request/response pattern but does not permit multiple messages per connection, which real framing would. Keeping this open for that.
