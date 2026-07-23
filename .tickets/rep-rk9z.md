---
id: rep-rk9z
status: closed
deps: []
links: []
created: 2026-07-23T06:30:15Z
type: bug
priority: 1
assignee: memgrafter
---
# Fix standard Jupyter kernel request timeout

## Notes

**2026-07-23T06:32:14Z**

Root cause: replmux hex-decoded hex-looking connection keys before HMAC, while Jupyter treats the JSON key as literal UTF-8 bytes. Standard kernels therefore discarded requests with invalid signatures. Preserving literal bytes made the existing binary execute 6 * 7 through an externally launched ipykernel with a non-hex key. Updated the Rust client, regression test, and bundled minimal kernel. No build/test command run per project policy.

**2026-07-23T06:33:13Z**

Test failure exposed a second boundary bug: the bundled kernel rewrote a supplied textual key using self.key.hex(), so its in-memory HMAC key no longer matched the rewritten connection document.

**2026-07-23T06:33:42Z**

Fixed connection serialization to retain the exact supplied textual key while using its UTF-8 bytes for HMAC. Random internally generated keys remain hex-serialized.
