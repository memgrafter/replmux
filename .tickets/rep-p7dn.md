---
id: rep-p7dn
status: closed
deps: []
links: []
created: 2026-08-18T12:21:23Z
type: bug
priority: 1
assignee: memgrafter
tags: [scripts, release]
---
# release.sh: non-static builds verify the wrong binary path

release.sh sets release_binary to cli/target/<host-triple>/release/replmux in all modes, but non-static mode runs build-and-test.sh without --target, so cargo places the binary at cli/target/release/replmux. otool verification then fails with 'can't open file'. Make the path conditional on static_mode.
