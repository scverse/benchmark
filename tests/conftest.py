"""Global configuration for all unit and integration tests."""

from __future__ import annotations

from typing import TYPE_CHECKING

import anyio
import pytest


if TYPE_CHECKING:
    from collections.abc import AsyncGenerator


@pytest.fixture(scope="session", autouse=True)
def anyio_backend() -> str:
    """Define backend for async tests run through the `anyio` plugin.

    It’s marked as autouse to tell pytest to use this plugin for async tests.
    """
    return "asyncio"


@pytest.fixture(autouse=True)
async def _timeout() -> AsyncGenerator[None, None]:
    """Timeout for tests."""
    with anyio.fail_after(0.2):
        yield
