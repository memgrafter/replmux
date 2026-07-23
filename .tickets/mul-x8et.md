---
id: mul-x8et
status: closed
deps: []
links: []
created: 2026-07-23T03:20:31Z
type: task
priority: 2
assignee: memgrafter
---
# Migrate service tests to httpx2

## Notes

**2026-07-23T03:21:06Z**

Replaced the service dev dependency httpx>=0.28,<1 with httpx2>=2.7,<3 and refreshed service/uv.lock. Synced the locked environment and ran service tests: 7 passed in 0.24s with no deprecation warning.
