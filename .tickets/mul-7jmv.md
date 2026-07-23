---
id: mul-7jmv
status: closed
deps: []
links: []
created: 2026-07-23T03:49:40Z
type: feature
priority: 0
assignee: memgrafter
---
# Bundle ZeroMQ into Rust CLI

## Notes

**2026-07-23T03:53:02Z**

Added Rust zmq 0.10 dependency, whose zmq-sys build uses zeromq-src to compile static libzmq. Added signed Jupyter control-channel shutdown using bundled ZMQ, HMAC reply validation, bundled-version unit test, and release-time otool/ldd rejection of dynamic libzmq. Updated lockfile/docs. Metadata, formatting, shell syntax, and diff checks pass; full clean release rerun required.
