from __future__ import annotations

import os
from typing import TYPE_CHECKING
from urllib.parse import urlparse

import rich.console


if TYPE_CHECKING:
    from typing import Never

    import taskiq


stderr = rich.console.Console(stderr=True)


def initial_broker() -> taskiq.AsyncBroker:
    return url2broker(os.environ.get("TASKIQ_BROKER", "amqp"))


def url2broker(queue: str) -> taskiq.AsyncBroker:
    try:
        url, scheme = (queue, urlparse(queue).scheme) if ":" in queue else (None, queue)
    except ValueError as e:
        msg = f"Invalid queue URL: {queue}"
        raise ValueError(msg) from e
    match scheme:
        case "amqp":
            try:
                from taskiq_aio_pika import AioPikaBroker
            except ImportError as e:
                import_exit("aio-pika", e)
            return AioPikaBroker(url)
        case _:
            msg = f"Unsupported queue: {scheme}"
            raise ValueError(msg)


def import_exit(mod: str, e: ImportError, *, extra: bool = False) -> Never:
    stderr.print(f"Failed to import {mod} module.", style="bold red")
    msg = (
        f"Try installing the {mod} extra via `pip install scverse-benchmark[{mod}]`"
        if extra
        else f"Try installing {mod} via `pip install {mod}`"
    )
    stderr.print(msg, style="bold")
    raise e
