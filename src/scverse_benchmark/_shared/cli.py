from __future__ import annotations

from typing import TYPE_CHECKING

from tap import Tap

if TYPE_CHECKING:
    from collections.abc import Sequence


class QueueArgs(Tap):
    pass


class RunnerArgs(Tap):
    pass


class Args(Tap):
    command: QueueArgs | RunnerArgs

    def configure(self) -> None:
        self.add_subparsers(required=True, title="Commands")
        self.add_subparser("queue", QueueArgs, help=QueueArgs.__doc__)
        self.add_subparser("runner", RunnerArgs, help=RunnerArgs.__doc__)


def main(args: Sequence[str] | None = None) -> None:
    args: Args = Args().parse_args(args)
    print({k: v for k, v in args.__dict__.items() if not k.startswith("_")})  # noqa: T201
