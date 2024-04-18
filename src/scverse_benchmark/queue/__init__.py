"""Queue code accepting webhooks and pushing events to a redis queue."""

from __future__ import annotations

from contextlib import asynccontextmanager
from typing import TYPE_CHECKING

import taskiq_fastapi
from fastapi import FastAPI
from githubkit.webhooks import parse

from .._shared import tasks


if TYPE_CHECKING:
    from collections.abc import AsyncGenerator
    from typing import Annotated

    from fastapi import Header, Request
    from githubkit.versions.latest.webhooks import EventNameType, PullRequestEvent

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


@app.post("/")
async def root(
    request: Request, x_github_event: Annotated[EventNameType, Header]
) -> str:
    """Receive webhook and potentially enqueue task."""
    if x_github_event != "pull_request":
        return f"Unknown event: {x_github_event}"
    body = await request.body()
    event: PullRequestEvent = parse("pull_request", body)
    if event.action not in {"opened", "reopened" "synchronize", "labeled"}:
        return "Unhandled action"
    if not any(lbl.name == "benchmark" for lbl in event.pull_request.labels):
        return "Not a benchmark PR"
    task = await tasks.compare.kiq()
    return f"Compare benchmarks... {task.task_id}"
