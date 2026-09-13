---
id: rep-79m5
status: closed
open: false
deps: []
links: []
created: 2026-09-13T17:36:19Z
type: task
priority: 2
assignee: memgrafter
---
# List manually validated Elixir and ROOT kernels across kernel catalogs

Add IElixir and ROOT to agent-facing validated-kernel inventories, retaining fourteen automated-matrix entries and distinguishing manual validation with limitations. Refresh xeus-cpp interruption caveat and historical validation report with subsequent ipykernel/xeus-cpp evidence. No new validation runs or runtime provisioning.

## Notes

**2026-09-13T17:40:12Z**

Added IElixir and ROOT/JupyROOT as manual-with-limitations entries across README, AGENTS, SKILL, agent catalog, blocked-kernel baseline, test README, and machine-readable catalog. Kept fourteen executable kernels unchanged; added two manual_validations records excluded by the existing installer. Updated xeus-cpp SIGINT/state-loss caveats and refreshed the original validation report with a separately labeled later ipykernel/xeus-cpp interruption run. Installed Replmux SKILL is hardlinked to repo SKILL, so it updated automatically. Static TOML/count/provisioner checks passed, all seven inventory documents include both additions, relative document links resolve, installed skill matches, and git diff --check passed. No new runtime validation, package installs, builds, or commits; unrelated worktree files untouched.
