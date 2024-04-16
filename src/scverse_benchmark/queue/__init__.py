"""Queue code accepting webhooks and pushing events to a redis queue."""

from __future__ import annotations

from typing import TYPE_CHECKING

from .._shared.tasks import Tasks

if TYPE_CHECKING:
    import taskiq


async def start(*, broker: taskiq.AsyncBroker) -> None:
    """Listen for webhooks and push events to a redis queue."""
    tasks = Tasks(broker)

    await broker.startup()
    # TODO(flying-sheep): loop and enqueue tasks  # noqa: TD003
    await tasks.compare.kiq()

    await broker.shutdown()
