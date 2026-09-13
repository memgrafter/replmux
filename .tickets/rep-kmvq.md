---
id: rep-kmvq
status: in_progress
open: true
deps: []
links: [rep-z9nu]
created: 2026-09-13T16:56:28Z
type: bug
priority: 1
assignee: memgrafter
---
# Restore single-file Pi REPL extension deployment

Inline abort-aware transport into replTool.ts so copying the extension as one file does not require transport.ts. Preserve focused transport regression fixtures and update the deployed dotfiles copy without unrelated changes.

## Notes

**2026-09-13T16:59:45Z**

Inlined the existing abort-aware transport into pi/extension/replTool.ts and removed the helper file (including its previously staged addition). Updated fixtures to load the actual inline section with native TypeScript stripping and added a relative-import guard. Updated single-file deployment docs. The reported memgrafter-pi-dotfiles extension is already a symlink to this repo file, so it receives the fix without edits in that repository. Node syntax checks passed for source, test file, and deployed symlink; git diff --check passed. No builds, automated test runs, or commits. Awaiting Pi reload confirmation before the requested minimal-worker sleep test; that test can verify prompt wait cancellation, not non-destructive cancellation of Python code.
