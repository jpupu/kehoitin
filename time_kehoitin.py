#!/usr/bin/env python

import timeit
import subprocess
import sys
from pathlib import Path
import socket

SOURCE = Path("example.prompt").read_bytes()
COMPILED = Path("example.compiled").read_bytes()
EXECUTED = (
    Path("example.executed")
    .read_bytes()
    .replace(b"/home/arto/src/kehoitin", bytes(Path.cwd()))
    .replace(b"localhost-live", socket.gethostname().encode())
)


def run(command: str, input: bytes, output: bytes) -> None:
    p = subprocess.run([executable, command], input=input, capture_output=True)
    if p.stdout != output:
        raise RuntimeError(
            f"running [{command}] failed:\n"
            f"OUT  {p.stdout!r}\n"
            f"GOAL {output!r}\n"
            f"STATUS {p.returncode}\n"
            f"STDERR {p.stderr.decode()}\n"
        )


def timed_run(command: str, input: bytes, output: bytes) -> None:
    t = timeit.Timer(
        "run(command, input, output)",
        setup="from __main__ import run",
        globals=dict(command=command, input=input, output=output),
    )
    count, time = t.autorange()
    print(
        f"{command}: {count} calls in {time * 1000:.0f} ms, {time / count * 1e6:.0f} μs/call"
    )


if len(sys.argv) != 2:
    print("usage: test_kehoitin.py PATH-TO-KEHOITIN-EXECUTABLE", file=sys.stderr)
    sys.exit(1)
executable = sys.argv[1]

timed_run("compile", SOURCE, COMPILED)
timed_run("execute", COMPILED, EXECUTED)
timed_run("interpret", SOURCE, EXECUTED)
