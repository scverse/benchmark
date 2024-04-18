from __future__ import annotations

from typing import TYPE_CHECKING

from scverse_benchmark._shared.broker_util import initial_broker


if TYPE_CHECKING:
    import taskiq


broker: taskiq.AsyncBroker = initial_broker()


@broker.task(task_name="compare")
async def compare() -> str:
    """Compare benchmarks."""
    return "Compare benchmarks..."
