#!/usr/bin/env python3
"""Accept signed OpenAI Codex release events and queue the local release agent."""

from __future__ import annotations

import hashlib
import hmac
import json
import os
import re
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any


CODEX_ROOT = Path(
    os.environ.get("CODEX_ROOT", "/Users/williamxu/Desktop/Projects/codex")
)
AGENT_SCRIPT = Path(
    os.environ.get(
        "CODEX_RELEASE_AGENT_SCRIPT",
        Path.home() / ".local/lib/codex/codex-release-agent.py",
    )
)
SECRET = os.environ.get("CODEX_RELEASE_WEBHOOK_SECRET", "")
HOST = os.environ.get("CODEX_RELEASE_WEBHOOK_HOST", "127.0.0.1")
PORT = int(os.environ.get("CODEX_RELEASE_WEBHOOK_PORT", "8765"))
WEBHOOK_PATH = os.environ.get("CODEX_RELEASE_WEBHOOK_PATH", "/github-release-webhook")
EXPECTED_REPO = os.environ.get("CODEX_RELEASE_WEBHOOK_REPO", "openai/codex")
CHANNEL = os.environ.get("CODEX_RELEASE_CHANNEL", "all")
TAG_RE = re.compile(r"^rust-v\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.\d+)?$")


def log(message: str) -> None:
    print(message, flush=True)


def verify_signature(body: bytes, signature: str, secret: str) -> bool:
    if not secret or not signature.startswith("sha256="):
        return False
    expected = hmac.new(secret.encode(), body, hashlib.sha256).hexdigest()
    return hmac.compare_digest(f"sha256={expected}", signature)


def release_decision(
    *,
    event: str,
    payload: dict[str, Any],
    expected_repo: str,
    channel: str,
) -> tuple[bool, str, str]:
    if event != "release":
        return False, "wrong event", ""
    if payload.get("action") != "published":
        return False, "release is not newly published", ""

    repository = str(payload.get("repository", {}).get("full_name", ""))
    if repository != expected_repo:
        return False, f"wrong repository: {repository}", ""

    release = payload.get("release", {})
    tag = str(release.get("tag_name", ""))
    if release.get("draft"):
        return False, "draft release", tag
    if not TAG_RE.fullmatch(tag):
        return False, f"unsupported release tag: {tag}", tag

    prerelease = bool(release.get("prerelease"))
    if channel == "stable" and prerelease:
        return False, "prerelease excluded by stable channel", tag
    if channel == "prerelease" and not prerelease:
        return False, "stable release excluded by prerelease channel", tag
    if channel not in {"all", "stable", "prerelease"}:
        return False, f"invalid release channel: {channel}", tag
    return True, "accepted", tag


def run_release_agent(tag: str, delivery: str) -> None:
    command = [
        str(AGENT_SCRIPT),
        "--release-tag",
        tag,
        "--delivery",
        delivery or "github-webhook",
        "--repository",
        EXPECTED_REPO,
        "--source-root",
        str(CODEX_ROOT),
    ]
    log(f"release agent start delivery={delivery} tag={tag}")
    result = subprocess.run(
        command,
        cwd=CODEX_ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.stdout:
        log(result.stdout.rstrip())
    if result.stderr:
        log(result.stderr.rstrip())
    log(f"release agent exit delivery={delivery} tag={tag} code={result.returncode}")


class Handler(BaseHTTPRequestHandler):
    server_version = "CodexReleaseWebhook/2.0"

    def do_GET(self) -> None:
        if self.path == "/health":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"ok\n")
            return
        self.send_error(404)

    def do_POST(self) -> None:
        if self.path != WEBHOOK_PATH:
            self.send_error(404)
            return

        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)
        event = self.headers.get("X-GitHub-Event", "")
        delivery = self.headers.get("X-GitHub-Delivery", "")
        signature = self.headers.get("X-Hub-Signature-256", "")

        if not verify_signature(body, signature, SECRET):
            log(f"reject delivery={delivery}: bad signature")
            self.send_error(401)
            return

        if event == "ping":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"pong\n")
            log(f"ping delivery={delivery}")
            return

        try:
            payload = json.loads(body.decode("utf-8"))
        except json.JSONDecodeError:
            self.send_error(400)
            return

        accepted, reason, tag = release_decision(
            event=event,
            payload=payload,
            expected_repo=EXPECTED_REPO,
            channel=CHANNEL,
        )
        if not accepted:
            self.send_response(202)
            self.end_headers()
            self.wfile.write(b"ignored\n")
            log(f"ignore delivery={delivery} tag={tag}: {reason}")
            return

        threading.Thread(
            target=run_release_agent,
            args=(tag, delivery),
            daemon=True,
        ).start()
        self.send_response(202)
        self.end_headers()
        self.wfile.write(b"release agent queued\n")

    def log_message(self, fmt: str, *args: object) -> None:
        return


def main() -> None:
    if not SECRET:
        raise SystemExit("CODEX_RELEASE_WEBHOOK_SECRET is required")
    if not AGENT_SCRIPT.is_file():
        raise SystemExit(f"missing release agent: {AGENT_SCRIPT}")
    server = ThreadingHTTPServer((HOST, PORT), Handler)
    log(
        f"listening host={HOST} port={PORT} path={WEBHOOK_PATH} "
        f"repo={EXPECTED_REPO} channel={CHANNEL}"
    )
    server.serve_forever()


if __name__ == "__main__":
    main()
