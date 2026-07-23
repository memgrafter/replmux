"""FastAPI application for runtime metadata."""

import os
from contextlib import asynccontextmanager
from pathlib import Path

from fastapi import FastAPI, HTTPException, Query, Response, status

from .models import (
    Runtime,
    RuntimeCreate,
    RuntimeList,
    RuntimeStatus,
    RuntimeUpdate,
)
from .store import (
    RuntimeNameConflictError,
    RuntimeNotFoundError,
    RuntimeStore,
)


DEFAULT_DATABASE_PATH = Path("~/.jupyter-repl/multirepl.db").expanduser()


def create_app(database_path: str | Path | None = None) -> FastAPI:
    resolved_path = Path(
        database_path
        or os.environ.get("MULTIREPL_DB_PATH", DEFAULT_DATABASE_PATH)
    ).expanduser()
    store = RuntimeStore(resolved_path)

    @asynccontextmanager
    async def lifespan(_: FastAPI):
        store.initialize()
        yield

    application = FastAPI(
        title="Multirepl Runtime API",
        summary="Durable runtime metadata for collaborative kernels.",
        version="0.1.0",
        lifespan=lifespan,
        openapi_tags=[
            {
                "name": "runtimes",
                "description": "Create and manage durable runtime records.",
            }
        ],
    )

    @application.get("/healthz", include_in_schema=False)
    def health() -> dict[str, str]:
        return {"status": "ok"}

    @application.post(
        "/v1/runtimes",
        response_model=Runtime,
        status_code=status.HTTP_201_CREATED,
        tags=["runtimes"],
        operation_id="createRuntime",
        summary="Create a runtime",
    )
    def create_runtime(request: RuntimeCreate) -> Runtime:
        try:
            return store.create(request)
        except RuntimeNameConflictError as error:
            raise HTTPException(
                status_code=status.HTTP_409_CONFLICT,
                detail=f"Runtime name already exists: {error}",
            ) from error

    @application.get(
        "/v1/runtimes",
        response_model=RuntimeList,
        tags=["runtimes"],
        operation_id="listRuntimes",
        summary="List runtimes",
    )
    def list_runtimes(
        limit: int = Query(default=50, ge=1, le=100),
        offset: int = Query(default=0, ge=0),
        runtime_status: RuntimeStatus | None = Query(default=None, alias="status"),
    ) -> RuntimeList:
        items, total = store.list(
            limit=limit,
            offset=offset,
            status=runtime_status,
        )
        return RuntimeList(items=items, total=total, limit=limit, offset=offset)

    @application.get(
        "/v1/runtimes/{runtime_id}",
        response_model=Runtime,
        tags=["runtimes"],
        operation_id="getRuntime",
        summary="Get a runtime",
    )
    def get_runtime(runtime_id: str) -> Runtime:
        return _get_runtime(store, runtime_id)

    @application.patch(
        "/v1/runtimes/{runtime_id}",
        response_model=Runtime,
        tags=["runtimes"],
        operation_id="updateRuntime",
        summary="Update a runtime",
    )
    def update_runtime(runtime_id: str, request: RuntimeUpdate) -> Runtime:
        try:
            return store.update(runtime_id, request)
        except RuntimeNotFoundError as error:
            raise _not_found(error) from error
        except RuntimeNameConflictError as error:
            raise HTTPException(
                status_code=status.HTTP_409_CONFLICT,
                detail=f"Runtime name already exists: {error}",
            ) from error

    @application.delete(
        "/v1/runtimes/{runtime_id}",
        status_code=status.HTTP_204_NO_CONTENT,
        tags=["runtimes"],
        operation_id="deleteRuntime",
        summary="Delete a runtime",
    )
    def delete_runtime(runtime_id: str) -> Response:
        try:
            store.delete(runtime_id)
        except RuntimeNotFoundError as error:
            raise _not_found(error) from error
        return Response(status_code=status.HTTP_204_NO_CONTENT)

    return application


def _get_runtime(store: RuntimeStore, runtime_id: str) -> Runtime:
    try:
        return store.get(runtime_id)
    except RuntimeNotFoundError as error:
        raise _not_found(error) from error


def _not_found(error: RuntimeNotFoundError) -> HTTPException:
    return HTTPException(
        status_code=status.HTTP_404_NOT_FOUND,
        detail=f"Runtime not found: {error}",
    )


app = create_app()
