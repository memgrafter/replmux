---
id: rep-4nen
status: open
open: true
deps: []
links: [rep-z4zo]
created: 2026-09-01T13:33:35Z
type: task
priority: 3
assignee: memgrafter
tags: [gepa, harness, skill, skill-description]
---
# Swap GEPA skill-optimization harness to a standard one (pi / flatmachines / bench)

## Why

`rep-z4zo` optimized the replmux SKILL.md description using a **custom
~350-line agent loop** (`~/code/prototyping/gepa_skill_harness/harness/agent.py`):
an OpenAI tool-calling loop whose `repl`/`repl-manage` tools are subprocess
wrappers around the replmux CLI, with a hand-written system prompt that
*imitates* how pi presents skills.

It worked (val avg 1.350 → 1.700, bash-python fallbacks 3 → 0), but it's a
prototype: the skill-presentation format is a best-effort copy of pi's, the
tool schemas are hand-copied, and the bash tool is a plain shell with a regex
guard. For future GEPA runs (other skills, other descriptions) and for
validating that a description actually triggers in the wild, we want a
**standard, trusted harness** so the measurement is credible and the loop code
isn't something we maintain.

## Options (try one or more; rank by trust + cost)

1. **pi harness (preferred — people trust it).** Run the real pi agent with
   the candidate description injected into its actual `<available_skills>`
   block, so skill presentation, tool schemas, and system prompt are exactly
   production. Highest fidelity; the rollout is what a real user's agent would
   do. Need: a way to drive pi headlessly per task (fresh session per rollout,
   capture the tool-call transcript for GEPA reflection, extract the final
   answer). The tmux-orchestration skill shows pi can be booted/pollable in
   separate windows; a headless/SDK mode would be cleaner.
2. **FlatMachine harness (works; Trent wants to burn it in).** The
   flatmachine-manager skill / flatmachine workflow configs can express the
   task → agent → verify flow. Good middle ground: standardized config,
   validated workflows, and it exercises a tool we want to get comfortable
   with. Need: confirm it can run an agent with a custom system prompt /
   skill list per candidate and emit per-task pass/fail + transcript.
3. **Bench harness (try it).** A terminal-bench-style harness (task dirs with
   instruction + verifier, containerized rollouts) — the shape GEPA's own
   terminal-bench adapter and gskill use. Heaviest option (docker per rollout)
   but the most standard for "agent does a task in a sandbox" and would let us
   reuse GEPA's published terminal-bench adapter patterns.

## Acceptance criteria

- [ ] Pick the harness (or run a bake-off: same 10 val tasks, same candidate
      descriptions from rep-z4zo, compare scores + wall time + flakiness).
- [ ] Wire the chosen harness into `gepa_skill_harness` as a swappable
      `run_agent()` backend (keep the current custom loop as `--harness custom`
      for reference).
- [ ] Re-run the rep-z4zo valset with the new harness and confirm the
      committed description still beats the seed (sanity: the ranking should
      survive the harness swap).
- [ ] Capture per-rollout transcripts in the format GEPA's
      `make_reflective_dataset` expects (Inputs / Generated Outputs / Feedback).

## Notes

- The gepa adapter side (`harness/gepa_adapter.py`) is harness-agnostic — only
  `run_agent()` / `Rollout` need to be reimplemented per backend.
- Cost note: pi and flatmachine rollouts on the local janus model are cheap;
  the bench/docker option is the expensive one — only worth it if we want
  container isolation or the standard terminal-bench task format.
- rep-z4zo's run artifacts (`runs/main1`, `runs/main2`) are the baseline to
  compare against after the swap.
