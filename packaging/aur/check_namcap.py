#!/usr/bin/env python3
"""Run namcap and fail on errors (namcap's exit status alone is insufficient)."""

import re
import subprocess
import sys


def main() -> None:
    errors = False
    for path in sys.argv[1:]:
        result = subprocess.run(["namcap", path], capture_output=True, text=True)
        output = result.stdout + result.stderr
        print(output, end="")
        errors |= result.returncode != 0 or bool(re.search(r"^[^\n]+ E: ", output, re.MULTILINE))
    if errors:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
