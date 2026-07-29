"""CLI tool with Click-like command groups, options, and callbacks."""

import functools
from typing import Optional


class Command:
    """Simple command decorator."""

    def __init__(self, name: Optional[str] = None) -> None:
        self.name = name

    def __call__(self, fn: callable) -> callable:
        fn.__command__ = self.name or fn.__name__
        return fn


class Group:
    """Command group with subcommands."""

    def __init__(self) -> None:
        self._commands: dict[str, callable] = {}

    def command(self, name: Optional[str] = None) -> callable:
        def decorator(fn: callable) -> callable:
            cmd_name = name or fn.__name__
            self._commands[cmd_name] = fn
            return fn

        return decorator

    def run(self, argv: Optional[list[str]] = None) -> None:
        args = argv or []
        if not args:
            print("Usage: <command> [options]")
            return
        cmd_name = args[0]
        cmd = self._commands.get(cmd_name)
        if cmd is None:
            print(f"Unknown command: {cmd_name}")
            return
        cmd(*args[1:])


@Command(name="greet")
def greet(name: str = "world", greeting: str = "Hello") -> str:
    return f"{greeting}, {name}!"


cli = Group()


@cli.command(name="add")
def add(a: int, b: int) -> int:
    return a + b


@cli.command(name="process")
def process(
    input_file: str,
    output_file: str = "out.txt",
    verbose: bool = False,
) -> None:
    if verbose:
        print(f"Processing {input_file} -> {output_file}")
    with open(input_file) as f:
        data = f.read()
    result = data.upper()
    with open(output_file, "w") as f:
        f.write(result)