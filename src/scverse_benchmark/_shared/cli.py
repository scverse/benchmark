from __future__ import annotations

import abc
from types import SimpleNamespace
from typing import Annotated

import typer

DEFAULT_REDIS_PORT = 6379

app = typer.Typer(no_args_is_help=True)


class Args(SimpleNamespace):
    queue: str


class Context(typer.Context, abc.ABC):
    obj: Args


def validate_addr(addr: str) -> str:
    if ":" in addr:
        _, port = addr.rsplit(":")
    else:
        port = DEFAULT_REDIS_PORT
    try:
        int(port)
    except ValueError:
        msg = f"Invalid port {port}"
        raise typer.BadParameter(msg) from None
    return addr


@app.callback()
def main(
    ctx: Context,
    queue: Annotated[
        str, typer.Option(help="Redis queue address", callback=validate_addr)
    ] = f"0.0.0.0:{DEFAULT_REDIS_PORT}",
) -> None:
    ctx.obj = Args(queue=queue)


@app.command()
def queue(ctx: Context) -> None:
    """Listen for webhooks and push events to a redis queue."""
    print(ctx.obj.queue)  # noqa: T201


@app.command()
def runner(ctx: Context) -> None:
    """Fetch events from a redis queue and run benchmarks."""
    print(ctx.obj.queue)  # noqa: T201
