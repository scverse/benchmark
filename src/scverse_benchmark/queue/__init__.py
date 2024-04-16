"""Queue code accepting webhooks and pushing events to a redis queue."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import taskiq


async def start(*, broker: taskiq.AsyncBroker) -> None:
    """Listen for webhooks and push events to a redis queue."""
    await broker.startup()
    # TODO(flying-sheep): loop and enqueue tasks  # noqa: TD003
