"""Runner code fetching events from a redis queue and running benchmarks."""

from __future__ import annotations

from typing import TYPE_CHECKING

from taskiq.receiver import Receiver

if TYPE_CHECKING:
    import taskiq


async def start(*, broker: taskiq.AsyncBroker) -> None:
    """Fetch events from a redis queue and run benchmarks."""
    recv = Receiver(broker)
    # TODO(flying-sheep): import tasks  # noqa: TD003
    await recv.listen()
