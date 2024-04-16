from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

    import taskiq
    from taskiq.abc.broker import _FuncParams, _ReturnType


def _fake_task(
    func: Callable[_FuncParams, _ReturnType],
) -> taskiq.AsyncTaskiqDecoratedTask[_FuncParams, _ReturnType]:
    """Fake task decorator. Made real in `__post_init__`."""
    func._fake_task = True  # noqa: SLF001
    return func  # type: ignore[return-value]


@dataclass
class Tasks:
    broker: taskiq.AsyncBroker

    def __post_init__(self) -> None:
        for name, cls_attr in vars(type(self)).items():
            if name.startswith("_") or not getattr(cls_attr, "_fake_task", False):
                continue
            func = getattr(self, name)
            setattr(self, name, self.broker.register_task(func, name))

    @_fake_task
    @staticmethod
    async def compare() -> None:
        """Compare benchmarks."""
        print("Compare benchmarks...")  # noqa: T201
