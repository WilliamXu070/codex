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


class WorkspacePreparationTests(unittest.TestCase):
    def test_retry_restores_existing_published_branch(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            remote = root / "origin.git"
            tag = "rust-v0.146.0-alpha.14"
            branch = "agent/upstream-0.146.0-alpha.14"

            subprocess.run(["git", "init", "--bare", "-q", remote], check=True)
            subprocess.run(
                ["git", "init", "-q", "-b", "main", source],
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Release Agent Test"],
                cwd=source,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.email", "release-agent@example.test"],
                cwd=source,
                check=True,
            )
            (source / "base.txt").write_text("base\n", encoding="utf-8")
            subprocess.run(["git", "add", "."], cwd=source, check=True)
            subprocess.run(
                ["git", "commit", "-qm", "base"],
                cwd=source,
                check=True,
            )
            subprocess.run(["git", "tag", tag], cwd=source, check=True)
            subprocess.run(
                ["git", "remote", "add", "origin", str(remote)],
                cwd=source,
                check=True,
            )
            subprocess.run(
                ["git", "push", "-q", "-u", "origin", "main"],
                cwd=source,
                check=True,
            )
            subprocess.run(
                ["git", "push", "-q", "origin", f"refs/tags/{tag}"],
                cwd=source,
                check=True,
            )
            subprocess.run(
                ["git", "switch", "-q", "-c", branch],
                cwd=source,
                check=True,
            )
            (source / "integrated.txt").write_text(
                "published integration\n",
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "."], cwd=source, check=True)
            subprocess.run(
                ["git", "commit", "-qm", "published integration"],
                cwd=source,
                check=True,
            )
            subprocess.run(
                ["git", "push", "-q", "-u", "origin", branch],
                cwd=source,
                check=True,
            )
            published_head = agent.git_output(source, "rev-parse", "HEAD")
            subprocess.run(
                ["git", "switch", "-q", "main"],
                cwd=source,
                check=True,
            )

            workspace, restored_branch, _, context_dir = agent.prepare_workspace(
                source_root=source,
                state_dir=root / "state",
                tag=tag,
                retry_failed=True,
                upstream_url=str(remote),
            )

            self.assertEqual(restored_branch, branch)
            self.assertEqual(
                agent.git_output(workspace, "branch", "--show-current"),
                branch,
            )
            self.assertEqual(
                agent.git_output(workspace, "rev-parse", "HEAD"),
                published_head,
            )
            self.assertTrue((workspace / "integrated.txt").is_file())
            self.assertTrue(context_dir.is_dir())


class WorkspaceVerificationTests(unittest.TestCase):
    def test_manifest_policy_failure_stops_preflight(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            workspace = Path(temp)
            for relative in (
                "scripts/test-codex-sound-path.sh",
                "william/audio/random-sound",
                "william/commands/sound",
                "william/transcribe/transcribe-command",
            ):
                path = workspace / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fixture\n", encoding="utf-8")

            with (
                mock.patch.object(agent, "git_output", return_value=""),
                mock.patch.object(
                    agent,
                    "workspace_version",
                    return_value="0.146.0-alpha.14",
                ),
                mock.patch.object(
                    agent,
                    "run_command",
                    side_effect=[
                        mock.DEFAULT,
                        mock.DEFAULT,
                        agent.ReleaseAgentError("manifest policy failed"),
                    ],
                ) as run_command,
            ):
                with self.assertRaisesRegex(
                    agent.ReleaseAgentError,
                    "manifest policy failed",
                ):
                    agent.verify_workspace(
                        workspace=workspace,
                        tag="rust-v0.146.0-alpha.14",
                        version="0.146.0-alpha.14",
                        skip_build=True,
                    )

            self.assertEqual(
                run_command.call_args_list,
                [
                    mock.call(
                        [
                            "git",
                            "merge-base",
                            "--is-ancestor",
                            "origin/main",
                            "HEAD",
                        ],
                        cwd=workspace,
                    ),
                    mock.call(
                        [
                            "git",
                            "merge-base",
                            "--is-ancestor",
                            "refs/tags/rust-v0.146.0-alpha.14",
                            "HEAD",
                        ],
                        cwd=workspace,
                    ),
                    mock.call(
                        [
                            sys.executable,
                            ".github/scripts/verify_cargo_workspace_manifests.py",
                        ],
                        cwd=workspace,
                    ),
                ],
            )


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
            required_ci_check="CI required",
            ci_timeout_seconds=10,
            ci_poll_interval_seconds=0,
            max_repair_attempts=2,
            skip_build=True,
            no_publish=True,
            no_activate=False,
            install_root=state / "releases",
            active_cli=state / "bin/codex",
            active_tui=state / "bin/codex-tui",
            current_link=state / "current",
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

    def test_release_is_published_merged_and_activated_once(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            workspace = root / "workspace"
            source.mkdir()
            workspace.mkdir()
            args = self.make_args(source, root / "state")
            args.no_publish = False
            installed = agent.InstalledRelease(
                release_dir=root / "release",
                cli=root / "release/debug/codex",
                tui=root / "release/debug/codex-tui",
                previous_cli="/previous/codex",
                previous_tui="/previous/codex-tui",
                previous_current="/previous",
            )

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
                mock.patch.object(agent, "run_codex_agent"),
                mock.patch.object(agent, "verify_with_repairs"),
                mock.patch.object(
                    agent,
                    "publish_branch",
                    return_value="https://example.test/pr/1",
                ) as publish,
                mock.patch.object(
                    agent,
                    "merge_pull_request",
                    return_value="abc123def456",
                ) as merge,
                mock.patch.object(
                    agent,
                    "wait_for_pull_request_ci",
                ) as wait_for_ci,
                mock.patch.object(
                    agent,
                    "install_active_cli",
                    return_value=installed,
                ) as install,
            ):
                first = agent.execute(args)
                second = agent.execute(args)

            self.assertEqual(first.status, "succeeded")
            self.assertEqual(first.merge_commit, "abc123def456")
            self.assertEqual(first.installed_cli, str(installed.cli))
            self.assertEqual(second.status, "skipped")
            publish.assert_called_once()
            wait_for_ci.assert_called_once_with(
                workspace=workspace,
                pr_url="https://example.test/pr/1",
                required_check="CI required",
                timeout_seconds=10,
                poll_interval_seconds=0,
            )
            merge.assert_called_once()
            install.assert_called_once()

    def test_failed_ci_prevents_merge_and_activation(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            workspace = root / "workspace"
            source.mkdir()
            workspace.mkdir()
            args = self.make_args(source, root / "state")
            args.no_publish = False

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
                mock.patch.object(agent, "run_codex_agent"),
                mock.patch.object(agent, "verify_with_repairs"),
                mock.patch.object(
                    agent,
                    "publish_branch",
                    return_value="https://example.test/pr/1",
                ),
                mock.patch.object(
                    agent,
                    "wait_for_pull_request_ci",
                    side_effect=agent.ReleaseAgentError("CI required failed"),
                ),
                mock.patch.object(agent, "merge_pull_request") as merge,
                mock.patch.object(agent, "install_active_cli") as install,
            ):
                with self.assertRaisesRegex(
                    agent.ReleaseAgentError,
                    "CI required failed",
                ):
                    agent.execute(args)

            merge.assert_not_called()
            install.assert_not_called()
            ledger = agent.ReleaseLedger(args.state_dir / "state.sqlite3")
            row = ledger.get(args.repository, args.release_tag)
            self.assertIsNotNone(row)
            assert row is not None
            self.assertEqual(row["status"], "failed")

    def test_retry_of_published_branch_skips_integration_agent(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            workspace = root / "workspace"
            source.mkdir()
            workspace.mkdir()
            args = self.make_args(source, root / "state")
            args.retry_failed = True
            args.no_publish = False
            ledger = agent.ReleaseLedger(args.state_dir / "state.sqlite3")
            ledger.claim(
                args.repository,
                args.release_tag,
                "first-attempt",
                retry_failed=False,
            )
            ledger.fail(args.repository, args.release_tag, error="CI failed")

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
                mock.patch.object(
                    agent,
                    "existing_pull_request_url",
                    return_value="https://example.test/pr/1",
                ),
                mock.patch.object(agent, "git_output", return_value=""),
                mock.patch.object(agent, "run_codex_agent") as run_agent,
                mock.patch.object(agent, "verify_with_repairs"),
                mock.patch.object(
                    agent,
                    "publish_branch",
                    return_value="https://example.test/pr/1",
                ),
                mock.patch.object(agent, "wait_for_pull_request_ci"),
                mock.patch.object(
                    agent,
                    "merge_pull_request",
                    return_value="abc123def456",
                ),
                mock.patch.object(
                    agent,
                    "install_active_cli",
                    return_value=agent.InstalledRelease(
                        release_dir=root / "release",
                        cli=root / "release/debug/codex",
                        tui=root / "release/debug/codex-tui",
                        previous_cli=None,
                        previous_tui=None,
                        previous_current=None,
                    ),
                ),
            ):
                result = agent.execute(args)

            self.assertEqual(result.status, "succeeded")
            self.assertEqual(result.pr_url, "https://example.test/pr/1")
            run_agent.assert_not_called()

    def test_interruption_marks_release_failed(self) -> None:
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
                mock.patch.object(
                    agent,
                    "run_codex_agent",
                    side_effect=KeyboardInterrupt,
                ),
            ):
                with self.assertRaises(KeyboardInterrupt):
                    agent.execute(args)

            ledger = agent.ReleaseLedger(args.state_dir / "state.sqlite3")
            row = ledger.get(args.repository, args.release_tag)
            self.assertIsNotNone(row)
            assert row is not None
            self.assertEqual(row["status"], "failed")


class PullRequestCiGateTests(unittest.TestCase):
    def test_successful_required_check_allows_merge(self) -> None:
        with mock.patch.object(
            agent,
            "read_pull_request_checks",
            return_value=[
                {
                    "name": "CI required",
                    "state": "SUCCESS",
                    "bucket": "pass",
                    "link": "https://example.test/check/1",
                }
            ],
        ):
            agent.wait_for_pull_request_ci(
                workspace=Path("/tmp/workspace"),
                pr_url="https://example.test/pr/1",
                required_check="CI required",
                timeout_seconds=10,
                poll_interval_seconds=0,
            )

    def test_failed_required_check_fails_closed(self) -> None:
        with mock.patch.object(
            agent,
            "read_pull_request_checks",
            return_value=[
                {
                    "name": "CI required",
                    "state": "FAILURE",
                    "bucket": "fail",
                    "link": "https://example.test/check/1",
                }
            ],
        ):
            with self.assertRaisesRegex(
                agent.ReleaseAgentError,
                "required CI check 'CI required' failed",
            ):
                agent.wait_for_pull_request_ci(
                    workspace=Path("/tmp/workspace"),
                    pr_url="https://example.test/pr/1",
                    required_check="CI required",
                    timeout_seconds=10,
                    poll_interval_seconds=0,
                )

    def test_missing_required_check_times_out_without_merging(self) -> None:
        with mock.patch.object(
            agent,
            "read_pull_request_checks",
            return_value=[],
        ):
            with self.assertRaisesRegex(
                agent.ReleaseAgentError,
                "timed out waiting for required CI check 'CI required'",
            ):
                agent.wait_for_pull_request_ci(
                    workspace=Path("/tmp/workspace"),
                    pr_url="https://example.test/pr/1",
                    required_check="CI required",
                    timeout_seconds=0,
                    poll_interval_seconds=0,
                )


class AgentSandboxTests(unittest.TestCase):
    def test_agent_ignores_user_config_and_scopes_filesystem_permissions(self) -> None:
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
            self.assertEqual(
                command[:5],
                [
                    "codex",
                    "exec",
                    "--ignore-user-config",
                    "--ephemeral",
                    "--json",
                ],
            )
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


class ActiveInstallTests(unittest.TestCase):
    def make_binary(self, path: Path, name: str, version: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            f"#!/bin/sh\nprintf '%s\\n' '{name} {version}'\n",
            encoding="utf-8",
        )
        path.chmod(0o755)

    def test_install_is_permanent_atomic_and_records_rollback(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            workspace = root / "workspace"
            built = workspace / "codex-rs/target/debug"
            self.make_binary(built / "codex", "codex-cli", "0.146.0-alpha.14")
            self.make_binary(
                built / "codex-tui",
                "codex-tui",
                "0.146.0-alpha.14",
            )

            old_release = root / "releases/old/debug"
            self.make_binary(old_release / "codex", "codex-cli", "0.144.6")
            self.make_binary(old_release / "codex-tui", "codex-tui", "0.144.6")
            active_cli = root / "bin/codex"
            active_tui = root / "bin/codex-tui"
            current = root / "current"
            active_cli.parent.mkdir()
            active_cli.symlink_to(old_release / "codex")
            active_tui.symlink_to(old_release / "codex-tui")
            current.symlink_to(old_release.parent)

            installed = agent.install_active_cli(
                workspace=workspace,
                tag="rust-v0.146.0-alpha.14",
                version="0.146.0-alpha.14",
                merge_commit="abc123def4567890",
                pr_url="https://example.test/pr/1",
                install_root=root / "releases",
                active_cli=active_cli,
                active_tui=active_tui,
                current_link=current,
            )

            self.assertEqual(installed.previous_cli, str(old_release / "codex"))
            self.assertEqual(
                installed.previous_tui,
                str(old_release / "codex-tui"),
            )
            self.assertEqual(os.readlink(active_cli), str(installed.cli))
            self.assertEqual(os.readlink(active_tui), str(installed.tui))
            self.assertEqual(os.readlink(current), str(installed.release_dir))
            metadata = (installed.release_dir / "release.json").read_text(
                encoding="utf-8"
            )
            self.assertIn(str(old_release / "codex"), metadata)
            self.assertIn(str(old_release / "codex-tui"), metadata)
            self.assertIn(
                "0.146.0-alpha.14",
                subprocess.check_output([active_cli, "--version"], text=True),
            )


if __name__ == "__main__":
    unittest.main()
