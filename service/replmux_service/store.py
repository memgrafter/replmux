"""SQLite persistence for runtimes."""

import json
import sqlite3
from datetime import UTC, datetime
from pathlib import Path
from uuid import uuid4

from .models import Runtime, RuntimeCreate, RuntimeStatus, RuntimeUpdate


class RuntimeNotFoundError(Exception):
    pass


class RuntimeNameConflictError(Exception):
    pass


class RuntimeStore:
    def __init__(self, database_path: str | Path):
        self.database_path = Path(database_path).expanduser()

    def initialize(self) -> None:
        self.database_path.parent.mkdir(parents=True, exist_ok=True)
        with self._connect() as connection:
            connection.execute("PRAGMA journal_mode=WAL")
            connection.execute(
                """
                CREATE TABLE IF NOT EXISTS runtimes (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
                    language TEXT NOT NULL,
                    environment_json TEXT NOT NULL,
                    snapshot_policy_json TEXT NOT NULL,
                    status TEXT NOT NULL,
                    worker_generation INTEGER NOT NULL,
                    revision INTEGER NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )
                """
            )

    def create(self, request: RuntimeCreate) -> Runtime:
        runtime_id = f"rt_{uuid4().hex}"
        now = datetime.now(UTC)
        values = (
            runtime_id,
            request.name,
            request.language,
            json.dumps(request.environment.model_dump(mode="json")),
            json.dumps(request.snapshot_policy.model_dump(mode="json")),
            RuntimeStatus.IDLE.value,
            0,
            1,
            now.isoformat(),
            now.isoformat(),
        )
        try:
            with self._connect() as connection:
                connection.execute(
                    """
                    INSERT INTO runtimes (
                        id, name, language, environment_json,
                        snapshot_policy_json, status, worker_generation,
                        revision, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    values,
                )
                row = connection.execute(
                    "SELECT * FROM runtimes WHERE id = ?", (runtime_id,)
                ).fetchone()
        except sqlite3.IntegrityError as error:
            raise RuntimeNameConflictError(request.name) from error
        return self._to_runtime(row)

    def list(
        self,
        *,
        limit: int,
        offset: int,
        status: RuntimeStatus | None = None,
    ) -> tuple[list[Runtime], int]:
        where = ""
        parameters: list[object] = []
        if status is not None:
            where = " WHERE status = ?"
            parameters.append(status.value)

        with self._connect() as connection:
            total = connection.execute(
                f"SELECT COUNT(*) FROM runtimes{where}", parameters
            ).fetchone()[0]
            rows = connection.execute(
                f"SELECT * FROM runtimes{where} ORDER BY created_at, id LIMIT ? OFFSET ?",
                [*parameters, limit, offset],
            ).fetchall()
        return [self._to_runtime(row) for row in rows], total

    def get(self, runtime_id: str) -> Runtime:
        with self._connect() as connection:
            row = connection.execute(
                "SELECT * FROM runtimes WHERE id = ?", (runtime_id,)
            ).fetchone()
        if row is None:
            raise RuntimeNotFoundError(runtime_id)
        return self._to_runtime(row)

    def update(self, runtime_id: str, request: RuntimeUpdate) -> Runtime:
        current = self.get(runtime_id)
        changes = {
            field: value
            for field, value in request.model_dump(exclude_unset=True, mode="json").items()
            if value is not None
        }
        if not changes:
            return current

        columns: list[str] = []
        values: list[object] = []
        for field, value in changes.items():
            if field == "environment":
                columns.append("environment_json = ?")
                values.append(json.dumps(value))
            elif field == "snapshot_policy":
                columns.append("snapshot_policy_json = ?")
                values.append(json.dumps(value))
            else:
                columns.append(f"{field} = ?")
                values.append(value)

        columns.extend(["revision = revision + 1", "updated_at = ?"])
        values.extend([datetime.now(UTC).isoformat(), runtime_id])

        try:
            with self._connect() as connection:
                cursor = connection.execute(
                    f"UPDATE runtimes SET {', '.join(columns)} WHERE id = ?",
                    values,
                )
                if cursor.rowcount == 0:
                    raise RuntimeNotFoundError(runtime_id)
        except sqlite3.IntegrityError as error:
            name = request.name or current.name
            raise RuntimeNameConflictError(name) from error
        return self.get(runtime_id)

    def delete(self, runtime_id: str) -> None:
        with self._connect() as connection:
            cursor = connection.execute(
                "DELETE FROM runtimes WHERE id = ?", (runtime_id,)
            )
        if cursor.rowcount == 0:
            raise RuntimeNotFoundError(runtime_id)

    def _connect(self) -> sqlite3.Connection:
        connection = sqlite3.connect(self.database_path, timeout=5)
        connection.row_factory = sqlite3.Row
        connection.execute("PRAGMA busy_timeout=5000")
        return connection

    @staticmethod
    def _to_runtime(row: sqlite3.Row) -> Runtime:
        return Runtime(
            id=row["id"],
            name=row["name"],
            language=row["language"],
            environment=json.loads(row["environment_json"]),
            snapshot_policy=json.loads(row["snapshot_policy_json"]),
            status=row["status"],
            worker_generation=row["worker_generation"],
            revision=row["revision"],
            created_at=datetime.fromisoformat(row["created_at"]),
            updated_at=datetime.fromisoformat(row["updated_at"]),
        )
