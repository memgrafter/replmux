"""API models for durable runtime metadata."""

from datetime import datetime
from enum import StrEnum

from pydantic import BaseModel, ConfigDict, Field


RUNTIME_NAME_PATTERN = r"^[A-Za-z0-9][A-Za-z0-9._-]*$"


class RuntimeStatus(StrEnum):
    IDLE = "idle"
    RUNNING = "running"
    HIBERNATED = "hibernated"
    FAILED = "failed"


class EnvironmentSpec(BaseModel):
    model_config = ConfigDict(extra="forbid")

    kind: str = Field(default="python", min_length=1, max_length=32)
    executable: str = Field(default="python3", min_length=1)
    digest: str | None = None


class SnapshotPolicy(BaseModel):
    model_config = ConfigDict(extra="forbid")

    interval_executions: int = Field(default=25, ge=1)
    mode: str = Field(default="logical", min_length=1, max_length=32)


class RuntimeCreate(BaseModel):
    model_config = ConfigDict(extra="forbid")

    name: str = Field(min_length=1, max_length=128, pattern=RUNTIME_NAME_PATTERN)
    language: str = Field(default="python", min_length=1, max_length=32)
    environment: EnvironmentSpec = Field(default_factory=EnvironmentSpec)
    snapshot_policy: SnapshotPolicy = Field(default_factory=SnapshotPolicy)


class RuntimeUpdate(BaseModel):
    model_config = ConfigDict(extra="forbid")

    name: str | None = Field(default=None, min_length=1, max_length=128, pattern=RUNTIME_NAME_PATTERN)
    language: str | None = Field(default=None, min_length=1, max_length=32)
    environment: EnvironmentSpec | None = None
    snapshot_policy: SnapshotPolicy | None = None
    status: RuntimeStatus | None = None


class Runtime(BaseModel):
    model_config = ConfigDict(extra="forbid")

    id: str
    name: str
    language: str
    environment: EnvironmentSpec
    snapshot_policy: SnapshotPolicy
    status: RuntimeStatus
    worker_generation: int
    revision: int
    created_at: datetime
    updated_at: datetime


class RuntimeList(BaseModel):
    model_config = ConfigDict(extra="forbid")

    items: list[Runtime]
    total: int
    limit: int
    offset: int
