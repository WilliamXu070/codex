#!/usr/bin/env python3

import os
from pathlib import Path
import subprocess
import sys
import unittest


class PackageTargetsTest(unittest.TestCase):
    def test_repository_root_defaults_to_source_checkout(self) -> None:
        repository_root = Path(__file__).resolve().parents[2]
        environment = os.environ.copy()
        environment.pop("CODEX_REPO_ROOT", None)

        completed = subprocess.run(
            [
                sys.executable,
                "-c",
                "from codex_package.targets import REPO_ROOT; print(REPO_ROOT)",
            ],
            cwd=repository_root / "scripts",
            env=environment,
            check=True,
            capture_output=True,
            text=True,
        )

        self.assertEqual(completed.stdout, f"{repository_root}\n")


if __name__ == "__main__":
    unittest.main()
