#!/usr/bin/env python

import subprocess
import json
from pathlib import Path
import sys


class CheckError(RuntimeError):
    pass


class Tester:
    def __init__(self, executable):
        self.executable = Path(executable).absolute()

    def setup(self) -> None:
        root = Path("/tmp/kehoitin-test")
        if root.exists():
            return

        root.mkdir(parents=True)
        (root / "foo").mkdir()
        (root / "gitty" / "sub").mkdir(parents=True)
        subprocess.call("git init", cwd=root / "gitty", shell=True)

    def check(self, mode: str, input: str | None, output_goal: str | None) -> str:
        if input is None:
            return output_goal

        p = subprocess.run(
            [self.executable, mode],
            input=input,
            cwd=self.cwd,
            capture_output=True,
            text=True,
        )
        if p.returncode != 0 or p.stderr:
            print("\x1b[31m \x1b[0m", end="\n")
            raise CheckError(
                f"Return code {p.returncode}:\n----stderr---\n{p.stderr}\n----stdout----\n{p.stdout}\n----\n"
            )
        output = p.stdout

        if output_goal is not None:
            if output == output_goal:
                print("\x1b[32m \x1b[0m", end="")
                return output
            else:
                print("\x1b[31m \x1b[0m", end="\n")
                print("expected:", repr(output_goal))
                print("got:     ", repr(output))
                raise CheckError("Check failed")
        else:
            print(" ", end="")
            return output

    def run_testcase(self, t: dict) -> None:
        source = t.get("source")
        compiled = t.get("compiled")
        executed = t.get("executed")
        if isinstance(source, list):
            source = "\n".join(source)
        if isinstance(compiled, list):
            compiled = "\n".join(compiled)
        if isinstance(executed, list):
            executed = "\n".join(executed)

        print("test:", t["name"], end="  ")

        self.cwd = t.get("working_directory") or Path.cwd()

        compiled = self.check("compile", source, compiled)
        if executed is not None:
            self.check("execute", compiled, executed)
        print("")

    def run_tests(self) -> None:
        tests = json.load(open("tests.json"))

        root = Path("/tmp/kehoitin-test")
        (root / "foo").mkdir(parents=True, exist_ok=True)

        try:
            for t in tests:
                self.run_testcase(t)
        except CheckError as ex:
            print()
            print(ex, file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: test_kehoitin.py PATH-TO-KEHOITIN-EXECUTABLE", file=sys.stderr)
        sys.exit(1)
    tester = Tester(sys.argv[1])
    tester.setup()
    tester.run_tests()
