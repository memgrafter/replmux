---
id: rep-ccfs
status: open
deps: [rep-x3ek, rep-sk5z]
links: []
created: 2026-07-23T19:01:39Z
type: epic
priority: 3
assignee: memgrafter
---
# Explore Flatmachines integration for recursive LLM workflows


"Go whole hog": use flatmachines to recursively launch recursive language model (RLM) style workflows via replmux, bypassing Pi's agent orchestration.

Requirements:
- Improve subagent visibility in flatmachines
- Potentially migrate to flatmachines CLI backend
- Wait for Mario's server/client split announcement (current blocker)
- If not available, prototype a basic CLI following flatmachines' standard pattern

Flatmachines CLI is itself a PROJECT — significant scope.

Open questions:
- Can replmux kernels serve as the execution substrate for flatmachines subagents?
- What's the minimal API surface flatmachines needs from replmux?
- Does this replace Pi's agent orchestration or complement it?

