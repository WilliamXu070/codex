from __future__ import annotations

import contextlib
import importlib.util
import io
import sys
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).with_name("verify_cargo_workspace_manifests.py")
SPEC = importlib.util.spec_from_file_location(
    "verify_cargo_workspace_manifests",
    SCRIPT,
)
assert SPEC and SPEC.loader
verify_cargo_workspace_manifests = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = verify_cargo_workspace_manifests
SPEC.loader.exec_module(verify_cargo_workspace_manifests)


class VerifyCargoWorkspaceManifestsTests(unittest.TestCase):
    def run_policy(self) -> tuple[int, str]:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = verify_cargo_workspace_manifests.main()
        return status, output.getvalue()

    def test_stale_code_mode_feature_exception_fails_until_removed(self) -> None:
        stale_exceptions = dict(
            verify_cargo_workspace_manifests.MANIFEST_FEATURE_EXCEPTIONS
        )
        stale_exceptions["codex-rs/code-mode/Cargo.toml"] = {
            "sandbox": ("v8/v8_enable_sandbox",)
        }

        with mock.patch.object(
            verify_cargo_workspace_manifests,
            "MANIFEST_FEATURE_EXCEPTIONS",
            stale_exceptions,
        ):
            stale_status, stale_output = self.run_policy()

        self.assertEqual(stale_status, 1)
        self.assertIn(
            "codex-rs/code-mode/Cargo.toml:\n"
            "  - remove the stale `[features]` exception from "
            "`MANIFEST_FEATURE_EXCEPTIONS`\n",
            stale_output,
        )

        corrected_status, corrected_output = self.run_policy()
        self.assertEqual((corrected_status, corrected_output), (0, ""))


if __name__ == "__main__":
    unittest.main()
