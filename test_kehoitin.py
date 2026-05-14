#!/usr/bin/env python

import subprocess
import json
from pathlib import Path
import sys
import tempfile


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

    def check(
        self,
        mode: str,
        input: str | None,
        output_goal: str | None,
        last_status: int | None,
    ) -> str:
        if input is None:
            return output_goal

        # Setup environment.
        env = {
            "LD_PRELOAD": Path("fakehostname/fakehostname.so").absolute(),
            "FAKEHOSTNAME": "testhost",
        }
        if last_status is not None:
            env["KEHOITIN_LAST_STATUS"] = str(last_status)

        # Setup input file.
        with tempfile.NamedTemporaryFile("w", delete_on_close=False) as infile:
            infile.write(input)
            infile.close()

            # Run the process.
            # Note that args[0] != executable, this reflects real-word usage where
            # the executable is in PATH and is run by filename alone.
            p = subprocess.Popen(
                ["kehoitin", mode, infile.name],
                executable=self.executable,
                cwd=self.cwd,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            stdout, stderr = p.communicate()

        if p.returncode != 0 or stderr:
            print("\x1b[31m \x1b[0m", end="\n")
            raise CheckError(
                f"Return code {p.returncode}:\n----stderr---\n{stderr}\n----stdout----\n{stdout}\n----\n"
            )

        if output_goal is not None:
            if stdout == output_goal:
                print("\x1b[32m \x1b[0m", end="")
                return stdout
            else:
                print("\x1b[31m \x1b[0m", end="\n")
                print("expected:", repr(output_goal))
                print("got:     ", repr(stdout))
                raise CheckError("Check failed")
        else:
            print(" ", end="")
            return stdout

    def run_testcase(self, t: dict) -> None:
        source = t.get("source")
        output = t.get("output")
        if isinstance(source, list):
            source = "\n".join(source)
        if isinstance(output, list):
            output = "\n".join(output)

        print("test:", t["name"], end="  ")

        self.cwd = t.get("working_directory") or Path.cwd()

        self.check("prompt", source, output, t.get("status"))
        print("")

    def run_tests(self) -> None:
        tests = json.load(open("testcases.json"))

        if not Path("fakehostname/fakehostname.so").exists():
            print("error: fakehostname/fakehostname.so not found!", file=sys.stderr)
            sys.exit(1)

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
