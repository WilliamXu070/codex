"""Tests for package target configuration."""

import os
import subprocess
import sys
import unittest
from pathlib import Path


class TargetsTest(unittest.TestCase):
    def test_repo_root_defaults_to_source_checkout(self) -> None:
        repo_root = Path(__file__).resolve().parents[2]
        environ = os.environ.copy()
        environ.pop("CODEX_REPO_ROOT", None)

        result = subprocess.run(
            [
                sys.executable,
                "-c",
                "from codex_package.targets import REPO_ROOT; print(REPO_ROOT)",
            ],
            cwd=repo_root / "scripts",
            env=environ,
            check=True,
            capture_output=True,
            text=True,
        )

        self.assertEqual(result.stdout.strip(), str(repo_root))


if __name__ == "__main__":
    unittest.main()
