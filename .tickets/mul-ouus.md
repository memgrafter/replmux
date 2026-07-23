---
id: mul-ouus
status: closed
deps: []
links: []
created: 2026-07-23T02:46:20Z
type: feature
priority: 1
assignee: memgrafter
---
# Add runtime CRUD API

Implement a standalone FastAPI runtime CRUD service with SQLite persistence, concise OpenAPI models, tests, and documentation under service/.

## Notes

**2026-07-23T02:50:11Z**

Implemented service/ FastAPI runtime CRUD with SQLite persistence, generated OpenAPI, uv lock, README, and 7 integration tests. Tests: `cd service && uv run pytest` -> 7 passed. Existing pi/extension/replTool.ts and other untracked work were not modified.
