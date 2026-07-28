from __future__ import annotations

import hashlib
import hmac
import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "codex-release-webhook-server.py"
SPEC = importlib.util.spec_from_file_location("codex_release_webhook", SCRIPT)
assert SPEC and SPEC.loader
webhook = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(webhook)


def payload(
    *,
    repository: str = "openai/codex",
    action: str = "published",
    tag: str = "rust-v0.146.0-alpha.14",
    prerelease: bool = True,
    draft: bool = False,
) -> dict[str, object]:
    return {
        "action": action,
        "repository": {"full_name": repository},
        "release": {
            "tag_name": tag,
            "prerelease": prerelease,
            "draft": draft,
        },
    }


class SignatureTests(unittest.TestCase):
    def test_signature_must_match(self) -> None:
        body = b'{"action":"published"}'
        secret = "secret"
        digest = hmac.new(secret.encode(), body, hashlib.sha256).hexdigest()
        self.assertTrue(webhook.verify_signature(body, f"sha256={digest}", secret))
        self.assertFalse(webhook.verify_signature(body, "sha256=wrong", secret))


class ReleaseDecisionTests(unittest.TestCase):
    def decide(
        self,
        value: dict[str, object],
        *,
        event: str = "release",
        channel: str = "all",
    ) -> tuple[bool, str, str]:
        return webhook.release_decision(
            event=event,
            payload=value,
            expected_repo="openai/codex",
            channel=channel,
        )

    def test_accepts_one_new_official_release(self) -> None:
        accepted, _, tag = self.decide(payload())
        self.assertTrue(accepted)
        self.assertEqual(tag, "rust-v0.146.0-alpha.14")

    def test_ignores_created_redelivery_wrong_repo_and_wrong_event(self) -> None:
        self.assertFalse(self.decide(payload(action="created"))[0])
        self.assertFalse(self.decide(payload(repository="WilliamXu070/codex"))[0])
        self.assertFalse(self.decide(payload(), event="push")[0])

    def test_channel_filter_does_not_promote_wrong_release_type(self) -> None:
        self.assertFalse(self.decide(payload(), channel="stable")[0])
        self.assertFalse(
            self.decide(
                payload(tag="rust-v0.145.0", prerelease=False),
                channel="prerelease",
            )[0]
        )
        self.assertTrue(
            self.decide(
                payload(tag="rust-v0.145.0", prerelease=False),
                channel="stable",
            )[0]
        )

    def test_rejects_non_codex_tag(self) -> None:
        self.assertFalse(self.decide(payload(tag="rusty-v8-v146.4.0"))[0])


if __name__ == "__main__":
    unittest.main()
