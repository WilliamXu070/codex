from __future__ import annotations

import importlib.util
import json
import os
import sys
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).with_name("check_ci_results.py")
SPEC = importlib.util.spec_from_file_location("check_ci_results", SCRIPT)
assert SPEC and SPEC.loader
check_ci_results = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = check_ci_results
SPEC.loader.exec_module(check_ci_results)


class CheckCiResultsTests(unittest.TestCase):
    def run_main(
        self,
        needs: dict[str, dict[str, str]],
        *,
        required: str | None = None,
    ) -> None:
        environment = {"NEEDS": json.dumps(needs)}
        if required is not None:
            environment["REQUIRED_DEPENDENCIES"] = required
        with mock.patch.dict(os.environ, environment, clear=True):
            check_ci_results.main()

    def test_all_dependencies_are_required_by_default(self) -> None:
        with self.assertRaises(SystemExit):
            self.run_main(
                {
                    "portable": {"result": "success"},
                    "upstream-only": {"result": "skipped"},
                }
            )

    def test_explicit_required_dependencies_ignore_expected_skips(self) -> None:
        self.run_main(
            {
                "fork-ci": {"result": "success"},
                "upstream-only": {"result": "skipped"},
            },
            required="fork-ci",
        )

    def test_missing_required_dependency_fails_closed(self) -> None:
        with self.assertRaises(SystemExit):
            self.run_main(
                {"fork-ci": {"result": "success"}},
                required="fork-ci,codespell",
            )


if __name__ == "__main__":
    unittest.main()
