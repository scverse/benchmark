from __future__ import annotations

import typer

app = typer.Typer()


@app.command()
def queue() -> None:
    """Listen for webhooks and push events to a redis queue."""


@app.command()
def runner() -> None:
    """Fetch events from a redis queue and run benchmarks."""
