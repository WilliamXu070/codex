import importlib.util
import os
import sys
import tempfile
import unittest
from pathlib import Path


AGENT_PATH = Path(__file__).parents[1] / "codex-release-agent.py"
SPEC = importlib.util.spec_from_file_location("codex_release_agent", AGENT_PATH)
assert SPEC is not None and SPEC.loader is not None
AGENT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = AGENT
SPEC.loader.exec_module(AGENT)


class ReleaseRetentionTests(unittest.TestCase):
    def test_keeps_required_releases_and_removes_older_directories(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            releases = [
                root / name for name in ("old-a", "old-b", "rollback", "active")
            ]
            for index, release in enumerate(releases):
                release.mkdir()
                (release / "artifact").write_text(release.name, encoding="utf-8")
                os.utime(release, (index + 1, index + 1))

            removed = AGENT.prune_installed_releases(
                install_root=root,
                required_releases=[releases[2], releases[3]],
            )

            self.assertEqual({path.name for path in removed}, {"old-a", "old-b"})
            self.assertEqual(
                {path.name for path in root.iterdir()},
                {"rollback", "active"},
            )

    def test_does_not_follow_or_remove_symlinked_directories(self) -> None:
        with (
            tempfile.TemporaryDirectory() as temporary,
            tempfile.TemporaryDirectory() as outside,
        ):
            root = Path(temporary)
            active = root / "active"
            active.mkdir()
            linked = root / "external"
            linked.symlink_to(outside, target_is_directory=True)

            removed = AGENT.prune_installed_releases(
                install_root=root,
                required_releases=[active],
                keep_count=1,
            )

            self.assertEqual(removed, [])
            self.assertTrue(linked.is_symlink())
            self.assertTrue(Path(outside).is_dir())

    def test_refuses_home_as_release_root(self) -> None:
        with self.assertRaises(AGENT.ReleaseAgentError):
            AGENT.prune_installed_releases(
                install_root=Path.home(),
                required_releases=[],
            )

    def test_removes_one_release_workspace(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            state_dir = Path(temporary) / "state"
            workspace = state_dir / "workspaces" / "rust-v1.2.3"
            target = workspace / "codex-rs/target/debug"
            target.mkdir(parents=True)
            (target / "codex").write_text("generated", encoding="utf-8")

            removed = AGENT.remove_release_workspace(
                state_dir=state_dir,
                workspace=workspace,
            )

            self.assertTrue(removed)
            self.assertFalse(workspace.exists())

    def test_refuses_workspace_outside_managed_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state_dir = root / "state"
            outside = root / "outside"
            outside.mkdir()

            with self.assertRaises(AGENT.ReleaseAgentError):
                AGENT.remove_release_workspace(
                    state_dir=state_dir,
                    workspace=outside,
                )

            self.assertTrue(outside.is_dir())

    def test_prunes_interrupted_release_workspaces(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            state_dir = Path(temporary) / "state"
            workspaces = state_dir / "workspaces"
            first = workspaces / "rust-v1.0.0"
            second = workspaces / "rust-v1.1.0"
            first.mkdir(parents=True)
            second.mkdir()

            removed = AGENT.prune_release_workspaces(state_dir=state_dir)

            self.assertEqual({item.name for item in removed}, {first.name, second.name})
            self.assertEqual(list(workspaces.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
