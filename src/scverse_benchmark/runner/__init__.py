"""Runner code fetching events from a redis queue and running benchmarks."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import taskiq


async def start(*, broker: taskiq.AsyncBroker) -> None:
    """Fetch events from a redis queue and run benchmarks."""
    print(broker)  # noqa: T201
