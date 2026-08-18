---
id: rep-nmrm
status: open
deps: []
links: []
created: 2026-08-18T11:09:48Z
type: feature
priority: 2
assignee: memgrafter
tags: [pi-extension, repl, tools]
---
# Keep read/write/edit in --repl mode; drop bash

The --repl flag in pi/extension/replTool.ts currently activates only [repl, repl-manage], removing the file tools. Change it to activate [read, write, edit, repl, repl-manage]: keep read/write/edit, drop bash, add the REPL tools. Update the flag description to match.

## Notes

**2026-08-18T11:34:18Z**

Superseded: instead of hard-coding [read, write, edit, repl, repl-manage], --repl now keeps every currently active tool except bash and adds repl/repl-manage, since the user loads extra tools (grep, find, etc.) that must survive.
