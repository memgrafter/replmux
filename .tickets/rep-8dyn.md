---
id: rep-8dyn
status: closed
deps: []
links: []
created: 2026-08-18T12:11:30Z
type: bug
priority: 2
assignee: memgrafter
tags: [cli, broker, timeout]
---
# Map broker client socket timeouts to friendly error messages

broker.rs send_request maps read/write io errors with plain error.to_string(). On macOS a 300s socket timeout surfaces as EAGAIN, so users see 'Resource temporarily unavailable (os error 35)' instead of a timeout message. Map TimedOut/WouldBlock to a friendly message, matching execute_socket in kernel.rs. Verified 2026-08-18: broker path cut at exactly 300s with the raw EAGAIN string.
