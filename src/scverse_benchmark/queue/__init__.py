"""Queue code accepting webhooks and pushing events to a redis queue."""

from __future__ import annotations

from contextlib import asynccontextmanager
from typing import TYPE_CHECKING

import taskiq_fastapi
from fastapi import FastAPI

from .._shared import tasks


if TYPE_CHECKING:
    from collections.abc import AsyncGenerator

taskiq_fastapi.init(tasks.broker, f"{__name__}:app")


@asynccontextmanager
async def lifespan(_app: FastAPI) -> AsyncGenerator[None, None]:
    """Lifespan manager."""
    if not tasks.broker.is_worker_process:
        await tasks.broker.startup()
    yield
    if not tasks.broker.is_worker_process:
        await tasks.broker.shutdown()


app = FastAPI(lifespan=lifespan)


@app.get("/")
async def root() -> str:
    """Receive webhook and potentially enqueue task."""
    task = await tasks.compare.kiq()
    return f"Compare benchmarks... {task.task_id}"
