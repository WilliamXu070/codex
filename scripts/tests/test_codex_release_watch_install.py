import os
import plistlib
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "install-codex-release-watch.sh"
REPO_ROOT = SCRIPT.parents[1]


class ReleaseWatchInstallTests(unittest.TestCase):
    def test_launchd_uses_canonical_codex_binary(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            home = root / "home"
            fake_bin = root / "bin"
            fake_bin.mkdir()
            launchctl = fake_bin / "launchctl"
            launchctl.write_text(
                '#!/bin/sh\nif [ "$1" = "list" ]; then\n  exit 1\nfi\nexit 0\n',
                encoding="utf-8",
            )
            launchctl.chmod(0o755)
            environment = os.environ.copy()
            environment.update(
                {
                    "CODEX_ROOT": str(REPO_ROOT),
                    "HOME": str(home),
                    "PATH": f"{fake_bin}:/usr/bin:/bin",
                }
            )

            subprocess.run(
                [str(SCRIPT), "install"],
                check=True,
                capture_output=True,
                text=True,
                env=environment,
            )

            plist_path = (
                home / "Library/LaunchAgents/com.williamxu.codex-release-watch.plist"
            )
            with plist_path.open("rb") as plist_file:
                plist = plistlib.load(plist_file)
            variables = plist["EnvironmentVariables"]

            self.assertEqual(
                variables["CODEX_RELEASE_AGENT_BINARY"],
                str(home / ".local/bin/codex"),
            )
            self.assertEqual(
                variables["PATH"].split(":", maxsplit=1)[0],
                str(home / ".local/bin"),
            )


if __name__ == "__main__":
    unittest.main()
