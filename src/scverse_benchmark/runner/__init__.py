"""Runner code fetching events from a redis queue and running benchmarks."""

from __future__ import annotations

import tracemalloc
from typing import TYPE_CHECKING

from taskiq.receiver import Receiver

# alternative: from taskiq.api import run_receiver_task
from .._shared.tasks import Tasks

if TYPE_CHECKING:
    import taskiq


async def start(*, broker: taskiq.AsyncBroker) -> None:
    """Fetch events from a redis queue and run benchmarks."""
    tracemalloc.start()  # debug async tasks

    Tasks(broker)  # import and register tasks
    broker.is_worker_process = True
    recv = Receiver(broker, run_starup=True, max_async_tasks=1)
    await recv.listen()
