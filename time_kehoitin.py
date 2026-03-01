#!/usr/bin/env python

import timeit
import subprocess
import sys

COMPILE_INPUT = b"""
color white blue
text "@"
func host
color yellow black slope
func cwd
color white black angle
text " "
color clear clear
"""

EXECUTE_INPUT = b"%`(host)::%`(cwd)>> "


def run_compile():
    subprocess.run([executable, "compile"], input=COMPILE_INPUT, capture_output=True)


def run_execute():
    subprocess.run([executable, "execute"], input=EXECUTE_INPUT, capture_output=True)


if len(sys.argv) != 2:
    print("usage: test_kehoitin.py PATH-TO-KEHOITIN-EXECUTABLE", file=sys.stderr)
    sys.exit(1)
executable = sys.argv[1]

t = timeit.Timer("run_compile()", setup="from __main__ import run_compile")
count, time = t.autorange()
print(
    f"compile: {count} calls in {time * 1000:.0f} ms, {time / count * 1e6:.0f} μs/call"
)

t = timeit.Timer("run_execute()", setup="from __main__ import run_execute")
count, time = t.autorange()
print(
    f"execute: {count} calls in {time * 1000:.0f} ms, {time / count * 1e6:.0f} μs/call"
)
