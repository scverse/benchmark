from __future__ import annotations

import abc
import asyncio
from types import SimpleNamespace
from typing import TYPE_CHECKING, Annotated, Never
from urllib.parse import urlparse

import typer
from rich.console import Console

if TYPE_CHECKING:
    import taskiq

stderr = Console(stderr=True)
app = typer.Typer(
    no_args_is_help=True,
    # See https://github.com/Textualize/rich/issues/1859
    pretty_exceptions_enable=False,
)


class Args(SimpleNamespace):
    broker: taskiq.AsyncBroker


class Context(typer.Context, abc.ABC):
    obj: Args


def url2broker(queue: str) -> taskiq.AsyncBroker:
    try:
        url, scheme = (queue, urlparse(queue).scheme) if ":" in queue else (None, queue)
    except ValueError:
        msg = f"Invalid queue URL: {queue}"
        raise typer.BadParameter(msg) from None
    match scheme:
        case "amqp":
            try:
                from taskiq_aio_pika import AioPikaBroker
            except ImportError:
                import_exit("aio-pika")
            return AioPikaBroker(url)
        case _:
            msg = f"Unsupported queue: {scheme}"
            raise typer.BadParameter(msg)


def import_exit(mod: str, *, extra: bool = False) -> Never:
    stderr.print(f"Failed to import {mod} module.", style="bold red")
    msg = (
        f"Try installing the {mod} extra via `pip install scverse-benchmark[{mod}]`"
        if extra
        else f"Try installing {mod} via `pip install {mod}`"
    )
    stderr.print(msg, style="bold")
    stderr.print_exception()
    raise typer.Exit(1)


@app.callback()
def main(
    ctx: Context,
    queue: Annotated[str, typer.Argument(help="Queue protocol or URL")],
) -> None:
    ctx.obj = Args(broker=url2broker(queue))


@app.command()
def queue(ctx: Context) -> None:
    """Listen for webhooks and push events to a queue."""
    try:
        from ..queue import start
    except ImportError:
        import_exit("queue")
    asyncio.run(start(broker=ctx.obj.broker))


@app.command()
def runner(ctx: Context) -> None:
    """Fetch events from a queue and run benchmarks."""
    try:
        from ..runner import start
    except ImportError:
        import_exit("runner")
    asyncio.run(start(broker=ctx.obj.broker))
