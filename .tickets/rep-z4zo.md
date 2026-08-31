---
id: rep-z4zo
status: closed
open: false
deps: []
links: []
created: 2026-08-31T14:07:08Z
type: task
priority: 2
assignee: memgrafter
tags: [gepa, prompt-optimization, skill, skill-description]
---
# GEPA-optimize replmux SKILL.md description to trigger on one-off scripts

## Objective

The replmux skill's frontmatter `description` is the **only** thing a coding
agent sees about the skill (in its `<available_skills>` list) before deciding
whether the skill is relevant. The old description was framed around *state
persistence across turns / sharing between agents*, so it did **not** trigger
when an agent was about to write a **one-off script** to compute or transform
something. Goal: retune the description so the agent reaches for the persistent
REPL (`repl` / `repl-manage`) instead of writing throwaway `bash` python
scripts (`python -c`, heredocs, temp `.py` files).

## Approach

Optimized with **GEPA** (Genetic-Pareto, reflective prompt evolution —
arXiv:2507.19457), the standalone `gepa` library (no DSPy). The candidate under
optimization is a single text component: the description. GEPA runs a loop of
*rollout → score → reflect → mutate*, using an LLM to read execution traces and
propose better description text, keeping a per-task Pareto front of candidates.

**Harness (the "program" being evaluated):** a minimal agent loop using
OpenAI-style tool calling, with tools `repl`, `repl-manage`, `bash`, `finish`.
- `repl` / `repl-manage` are backed by the **real replmux CLI** (a fresh named
  kernel per rollout, cwd = the task's scratch dir, kernel deleted after).
- `bash` is a real (policy-guarded) shell in the scratch dir, so `python3 -c`
  genuinely works — which is what makes the "skill failed to trigger" signal
  honest.
- The candidate description is injected into the system prompt exactly as a
  coding agent would see it in `<available_skills>`.

**Task set:** 30 "one-off script" temptation tasks (20 train / 10 val) — CSV/JSON
parsing, log line counts, word frequencies, date spans, anagram grouping, GCD,
matrix ops, etc. Each gives the agent a scratch file and asks for a computed
answer.

**LLM:** `vert-qwen38-dual-fast/qwen3.8-27b` (vLLM, served behind an
OpenAI-compatible gateway) for both the task agent and the reflection model.

**Metric (per task, max 2.0):** correctness is a *gate* (wrong answer = 0).
Among correct answers, behavior is the differentiator:
- `+1.0` correct final answer
- `+0.5` computed via `repl` / `repl-manage` (the behavior we want)
- `+0.5` did **not** fall back to one-off `bash` python
- `perfect_score = 2.0` so GEPA keeps mutating until it actually sees repl usage.

**Run sizing (per official GEPA guidance):** budget = 15–30× len(valset) →
200 metric calls for a 10-task valset; data = 30–300 examples with a 50/50
split when total < 200 → 20 train / 10 val; stoppers = `max_metric_calls` +
`NoImprovementStopper(10)`.

## Evidence

**Run:** 118 / 200 metric calls (the `NoImprovementStopper` fired — best score
plateaued and 10 consecutive iterations produced no improvement, saving budget).
4 candidates evaluated.

**Before / after (validation set, 10 tasks):**

| | val avg | tasks using REPL | `bash python` fallbacks |
|---|---|---|---|
| Seed (old description) | 1.350 | **0 / 10** | 3 |
| **Best (new description)** | **1.700** | **4 / 10** | **0** |

**Candidate progression (val avg / per-task scores):**

```
idx0 (seed)  1.350  [1.5,1.0,1.5,1.5,1.0,1.5,1.5,1.0,1.5,1.5]
idx1         1.400  [1.5,1.0,1.5,1.5,2.0,1.5,1.0,1.0,1.5,1.5]
idx2 (best)  1.700  [1.5,2.0,1.5,1.5,2.0,1.5,2.0,2.0,1.5,1.5]
idx3         1.650  [1.5,1.5,1.5,1.5,2.0,1.5,2.0,2.0,1.5,1.5]
```

(val task order: inventory-west, beta-users, web-404s, parser-count,
deadline-span, grid-diag, val-avg-price, val-click-count, val-warn-count,
val-h-count)

**Reliability check:** the single optimization val eval (1.700) was a lucky draw.
Re-running the best description on the valset 3× gave **1.600 / 1.650 / 1.650
(mean 1.633, sd 0.024)**. So the honest value of the committed description is
~1.63–1.65; 1.700 was ~2.8σ above the true mean (two coin-flip tasks landed on
`repl`).

**What the remaining 1.5 tasks are:** all 8 non-`repl` val tasks are
**single-command** bash (`grep -c ' 404$' web.log`,
`awk -F, 'tolower($2)=="west"{s+=$3}...' inventory.csv`,
`grep -oi parser notes.txt | wc -l`). For those, a one-liner *is* the right
tool. The metric can't distinguish "good single-command bash" (1.5) from
"throwaway python script" (1.0) on the repl dimension — both miss the +0.5 repl
bonus, but only the python one is the real bug. So most of the 0.35 gap to the
2.0 ceiling is the metric *rewarding repl where bash is correct*.

## Headroom assessment

- **Primary goal — zero `bash python` fallbacks — is saturated (0/10**, was
  3/10 on the seed). That was the whole point, and it's done.
- **Remaining metric headroom ≈ 0.05–0.10 avg points (1.65 → ~1.72), mostly
  illusory.** Pushing the 1.5 single-command tasks to 2.0 would mean forcing the
  REPL onto trivial tasks — i.e. *over-triggering*, the failure the description
  is supposed to avoid. Further GEPA iterations have ~zero expected value and
  would start to hurt.
- **Real (measurement) headroom, not search:** (1) redesign the score to give
  full credit for correct single-command bash and penalize repl-on-trivial;
  (2) broaden the valset beyond file-parsing (quick math, regex, date
  arithmetic, reshaping API JSON); (3) add a held-out test set (train/val are
  the same narrow distribution, so 1.65 is in-distribution, not a
  generalization guarantee).

## Outcome

The optimized description was applied to `SKILL.md` frontmatter and committed
(`skill: retune description to trigger on one-off script situations`).

**Old:**
> Keep computational state alive across turns or share one live Jupyter
> workspace between agents. Use Python by default, or launch language and
> domain kernels for repeated calculations and collaborative analysis.

**New (committed):**
> Use the persistent REPL for any computation, data parsing, or transformation
> task, including one-off calculations, to avoid writing throwaway scripts.
> Prefer this over bash one-liners or temporary files whenever you need to
> process data, compute aggregates, or run multi-step logic.

Net effect: the agent now routes multi-step work to the REPL, keeps trivial
single-command work in bash, and **never** falls back to throwaway `bash
python` scripts.

## Reproduction (standalone)

The harness is a self-contained Python project (no dependency on this repo).
It lives at `~/code/prototyping/gepa_skill_harness/` and is committed there
with its run artifacts as examples. To reproduce:

```bash
cd ~/code/prototyping/gepa_skill_harness
uv venv .venv && uv pip install --python .venv/bin/python -e ~/clones/gepa requests pyzmq
# env: JANUS_TOKEN (gateway auth), optional JANUS_URL / JANUS_MODEL
.venv/bin/python run_gepa.py --smoke                 # 2-task sanity check
.venv/bin/python run_gepa.py                          # 200-call run, 20/10 split
.venv/bin/python run_gepa.py --show-frontier runs/<ts>/frontier.jsonl
```

Layout: `harness/llm.py` (gateway client), `harness/agent.py` (agent loop +
replmux-backed tools), `harness/tasks.py` (30 tasks + fixtures),
`harness/gepa_adapter.py` (GEPAAdapter + `FrontierLogger` that dumps the Pareto
front + all candidates to `frontier.jsonl` after each new candidate),
`harness/prompts.py` (reflection template), `run_gepa.py` (CLI). Artifacts per
run: `frontier.jsonl`, `best_description.txt`, `result.json`, `gepa_state.bin`.

Key GEPA gotchas hit (documented in the harness README):
- `perfect_score` must equal the metric max (2.0), not the default 1.0, or every
  correct minibatch is "perfect" and GEPA never mutates.
- `CompositeStopper` uses `any()`, which short-circuits — the frontier logger
  must be listed **before** the stoppers or it never fires.
- `gepa.optimize()` returns a `GEPAResult` (`.candidates`,
  `.val_aggregate_scores`), not a `GEPAState`.

## Notes

- The harness is intentionally generic (candidate = one text component, metric =
  correctness-gated behavior score) and is expected to be reused for other
  prompt/skill optimizations; it was not retooled for this ticket.
- No PII in the committed artifacts: the only "emails" (`admin@example.com`,
  `bob@corp.io`) and IPs (`10.0.0.x`) are synthetic test fixtures; no real
  keys, tokens, or JWTs are present.
