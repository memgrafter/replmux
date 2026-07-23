# Replmux Runtime API

A small FastAPI service for durable runtime metadata. It implements only runtime CRUD; it does not start kernels or expose executions, branches, snapshots, or attachments.

## Run

```bash
cd service
uv sync --dev
uv run uvicorn replmux_service.app:app --reload
```

The SQLite database defaults to `~/.jupyter-repl/replmux.db`. Override it with:

```bash
REPLMUX_DB_PATH=/tmp/replmux.db uv run uvicorn replmux_service.app:app
```

OpenAPI is available at `http://127.0.0.1:8000/openapi.json`, with interactive documentation at `/docs`.

## API

### Create

```bash
curl -X POST http://127.0.0.1:8000/v1/runtimes \
  -H 'content-type: application/json' \
  -d '{"name":"analysis"}'
```

### List

```bash
curl 'http://127.0.0.1:8000/v1/runtimes?limit=50&offset=0'
```

Optional filter: `?status=running`.

### Get

```bash
curl http://127.0.0.1:8000/v1/runtimes/rt_ID
```

### Update

```bash
curl -X PATCH http://127.0.0.1:8000/v1/runtimes/rt_ID \
  -H 'content-type: application/json' \
  -d '{"status":"running"}'
```

### Delete

```bash
curl -X DELETE http://127.0.0.1:8000/v1/runtimes/rt_ID
```

## Test

```bash
cd service
uv run pytest
```
