"""Runner code fetching events from a redis queue and running benchmarks."""

from __future__ import annotations

import sys
import tracemalloc
from typing import Never

import anyio.abc
import taskiq
from taskiq.receiver import Receiver

from .._shared import tasks


async def start(*, task_status: anyio.abc.TaskStatus | None = None) -> Never:
    """Fetch events from a redis queue and run benchmarks.

    We’re not using the worker CLI here because we deliberately
    only want one in-process worker.
    """
    tracemalloc.start()  # debug async tasks

    tasks.broker.is_worker_process = True
    if task_status is not None:
        tasks.broker.add_event_handler(
            taskiq.TaskiqEvents.WORKER_STARTUP, task_status.started
        )

    recv = Receiver(tasks.broker, run_starup=True, max_async_tasks=1)
    await recv.listen()


def cli() -> Never:  # type: ignore[return]
    """Run runner until interrupted (Ctrl+C)."""
    try:
        anyio.run(start)
    except KeyboardInterrupt:
        sys.exit()
