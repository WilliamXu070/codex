from __future__ import annotations

import argparse
import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


SCRIPT = Path(__file__).parents[1] / "codex-release-agent.py"
SPEC = importlib.util.spec_from_file_location("codex_release_agent", SCRIPT)
assert SPEC and SPEC.loader
agent = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = agent
SPEC.loader.exec_module(agent)


class ReleaseLedgerTests(unittest.TestCase):
    def test_duplicate_success_is_not_claimed_again(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            ledger = agent.ReleaseLedger(Path(temp) / "state.sqlite3")
            first = ledger.claim(
                "openai/codex",
                "rust-v0.146.0-alpha.14",
                "delivery-1",
                retry_failed=False,
            )
            self.assertTrue(first.acquired)
            ledger.complete(
                "openai/codex",
                "rust-v0.146.0-alpha.14",
                branch="agent/upstream-0.146.0-alpha.14",
                workspace=Path(temp) / "workspace",
                pr_url="https://example.test/pr/1",
            )

            duplicate = ledger.claim(
                "openai/codex",
                "rust-v0.146.0-alpha.14",
                "delivery-2",
                retry_failed=False,
            )
            self.assertFalse(duplicate.acquired)
            self.assertEqual(duplicate.status, "succeeded")
            self.assertEqual(duplicate.attempts, 1)

    def test_failed_release_requires_explicit_retry(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            ledger = agent.ReleaseLedger(Path(temp) / "state.sqlite3")
            ledger.claim(
                "openai/codex",
                "rust-v0.146.0-alpha.14",
                "delivery-1",
                retry_failed=False,
            )
            ledger.fail(
                "openai/codex",
                "rust-v0.146.0-alpha.14",
                error="conflict",
            )

            automatic = ledger.claim(
                "openai/codex",
                "rust-v0.146.0-alpha.14",
                "delivery-2",
                retry_failed=False,
            )
            self.assertFalse(automatic.acquired)
            self.assertEqual(automatic.status, "failed")

            manual = ledger.claim(
                "openai/codex",
                "rust-v0.146.0-alpha.14",
                "manual-retry",
                retry_failed=True,
            )
            self.assertTrue(manual.acquired)
            self.assertEqual(manual.attempts, 2)


class ReleaseValidationTests(unittest.TestCase):
    def test_accepts_stable_and_prerelease_rust_tags(self) -> None:
        self.assertEqual(
            agent.validate_release("openai/codex", "rust-v0.145.0"),
            "0.145.0",
        )
        self.assertEqual(
            agent.validate_release(
                "openai/codex",
                "rust-v0.146.0-alpha.14",
            ),
            "0.146.0-alpha.14",
        )

    def test_rejects_wrong_repository_and_non_release_tags(self) -> None:
        with self.assertRaises(agent.ReleaseAgentError):
            agent.validate_release(
                "WilliamXu070/codex",
                "rust-v0.146.0-alpha.14",
            )
        with self.assertRaises(agent.ReleaseAgentError):
            agent.validate_release("openai/codex", "rusty-v8-v146.4.0")


class ExecuteDeduplicationTests(unittest.TestCase):
    def make_args(self, source: Path, state: Path) -> argparse.Namespace:
        return argparse.Namespace(
            source_root=source,
            state_dir=state,
            repository="openai/codex",
            release_tag="rust-v0.146.0-alpha.14",
            delivery="test-delivery",
            retry_failed=False,
            upstream_url="https://github.com/openai/codex.git",
            codex_binary="codex",
            timeout_seconds=10,
            max_repair_attempts=2,
            skip_build=True,
            no_publish=True,
        )

    def test_duplicate_execute_launches_codex_once(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            workspace = root / "workspace"
            source.mkdir()
            workspace.mkdir()
            args = self.make_args(source, root / "state")

            with (
                mock.patch.object(agent, "source_fingerprint", return_value="same"),
                mock.patch.object(
                    agent,
                    "prepare_workspace",
                    return_value=(
                        workspace,
                        "agent/upstream-0.146.0-alpha.14",
                        "abc123",
                        workspace / ".codex-release-context",
                    ),
                ),
                mock.patch.object(agent, "run_codex_agent") as run_agent,
                mock.patch.object(agent, "verify_with_repairs"),
            ):
                first = agent.execute(args)
                second = agent.execute(args)

            self.assertEqual(first.status, "succeeded")
            self.assertEqual(second.status, "skipped")
            run_agent.assert_called_once()


class AgentSandboxTests(unittest.TestCase):
    def test_agent_can_write_git_metadata_and_read_rust_toolchain(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            workspace = root / "workspace"
            state = root / "state"
            cargo_home = root / "cargo"
            rustup_home = root / "rustup"
            workspace.mkdir()

            with (
                mock.patch.dict(
                    os.environ,
                    {
                        "CARGO_HOME": str(cargo_home),
                        "RUSTUP_HOME": str(rustup_home),
                    },
                ),
                mock.patch.object(agent, "run_command") as run_command,
            ):
                agent.run_codex_agent(
                    workspace=workspace,
                    prompt="integrate",
                    state_dir=state,
                    tag="rust-v0.146.0-alpha.14",
                    codex_binary="codex",
                    timeout_seconds=10,
                )

            command = run_command.call_args.args[0]
            filesystem_config = command[command.index("-c", 6) + 1]
            git_config = Path.home() / ".gitconfig"
            git_config_dir = Path.home() / ".config/git"
            self.assertIn(f'"{cargo_home}"="read"', filesystem_config)
            self.assertIn(f'"{rustup_home}"="read"', filesystem_config)
            self.assertIn(f'"{git_config}"="read"', filesystem_config)
            self.assertIn(f'"{git_config_dir}"="read"', filesystem_config)
            self.assertIn('".git"="write"', filesystem_config)
            self.assertIn('".codex"="write"', filesystem_config)
            self.assertIn('".agents"="write"', filesystem_config)


class RepairLoopTests(unittest.TestCase):
    def test_validation_failure_gets_one_bounded_repair(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            workspace = root / "workspace"
            source = root / "source"
            workspace.mkdir()
            source.mkdir()

            with (
                mock.patch.object(agent, "verify_source_unchanged"),
                mock.patch.object(
                    agent,
                    "verify_workspace",
                    side_effect=[agent.ReleaseAgentError("compile failed"), None],
                ) as verify,
                mock.patch.object(agent, "run_codex_agent") as run_agent,
            ):
                agent.verify_with_repairs(
                    workspace=workspace,
                    source_root=source,
                    source_fingerprint_before="same",
                    state_dir=root / "state",
                    tag="rust-v0.146.0-alpha.14",
                    version="0.146.0-alpha.14",
                    branch="agent/upstream-0.146.0-alpha.14",
                    codex_binary="codex",
                    timeout_seconds=10,
                    skip_build=False,
                    max_repair_attempts=2,
                )

            self.assertEqual(verify.call_count, 2)
            run_agent.assert_called_once()
            self.assertEqual(
                run_agent.call_args.kwargs["log_suffix"],
                "repair-1",
            )

    def test_generated_lockfile_is_committed_and_nothing_else(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            workspace = Path(temp)
            lockfile = workspace / "codex-rs/Cargo.lock"
            lockfile.parent.mkdir()
            lockfile.write_text("before\n", encoding="utf-8")
            subprocess.run(["git", "init", "-q"], cwd=workspace, check=True)
            subprocess.run(
                ["git", "config", "user.name", "Release Agent Test"],
                cwd=workspace,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.email", "release-agent@example.test"],
                cwd=workspace,
                check=True,
            )
            subprocess.run(["git", "add", "."], cwd=workspace, check=True)
            subprocess.run(
                ["git", "commit", "-qm", "initial"],
                cwd=workspace,
                check=True,
            )
            lockfile.write_text("after\n", encoding="utf-8")

            agent.commit_generated_lockfile(workspace)

            self.assertEqual(
                agent.git_output(workspace, "status", "--porcelain"),
                "",
            )
            self.assertEqual(
                agent.git_output(workspace, "log", "-1", "--format=%s"),
                "Update Cargo lockfile for integrated release",
            )


if __name__ == "__main__":
    unittest.main()
