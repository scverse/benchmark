"""Test basic end-to-end functionality."""

from __future__ import annotations

from typing import TYPE_CHECKING, cast

import anyio
import pytest
from asgi_lifespan import LifespanManager
from httpx import ASGITransport, AsyncClient

from scverse_benchmark import queue, runner


if TYPE_CHECKING:
    from collections.abc import AsyncGenerator


@pytest.fixture()
async def client() -> AsyncGenerator[AsyncClient, None]:
    """Test client."""
    transport = ASGITransport(app=queue.app)  # type: ignore[call-arg]
    async with (
        LifespanManager(queue.app),
        AsyncClient(transport=transport, base_url="http://test") as ac,
    ):
        yield ac


@pytest.fixture()
async def _runner(client: AsyncClient) -> AsyncGenerator[None, None]:
    """Start runner."""
    del client  # client needs to start up first
    async with anyio.create_task_group() as tg:
        await tg.start(runner.start)
        yield
        tg.cancel_scope.cancel()


@pytest.mark.usefixtures("_runner")
async def test_basic(client: AsyncClient) -> None:
    """Test basic functionality."""
    resp = await client.get("/")
    resp_des = cast(str, resp.json())
    assert resp_des.startswith("Compare benchmarks...")
