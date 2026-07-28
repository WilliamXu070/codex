#!/usr/bin/env python3
import hashlib
import hmac
import json
import os
import subprocess
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


CODEX_ROOT = Path(os.environ.get("CODEX_ROOT", "/Users/williamxu/Desktop/Projects/codex"))
UPDATE_SCRIPT = Path(os.environ.get("CODEX_UPDATE_SCRIPT", CODEX_ROOT / "scripts/update-codex-local.sh"))
SECRET = os.environ.get("CODEX_RELEASE_WEBHOOK_SECRET", "")
HOST = os.environ.get("CODEX_RELEASE_WEBHOOK_HOST", "127.0.0.1")
PORT = int(os.environ.get("CODEX_RELEASE_WEBHOOK_PORT", "8765"))
WEBHOOK_PATH = os.environ.get("CODEX_RELEASE_WEBHOOK_PATH", "/github-release-webhook")
EXPECTED_REPO = os.environ.get("CODEX_RELEASE_WEBHOOK_REPO", "")

update_lock = threading.Lock()


def log(message: str) -> None:
    print(message, flush=True)


def verify_signature(body: bytes, signature: str) -> bool:
    if not SECRET or not signature.startswith("sha256="):
        return False
    expected = hmac.new(SECRET.encode(), body, hashlib.sha256).hexdigest()
    return hmac.compare_digest(f"sha256={expected}", signature)


def run_update(tag: str, delivery: str) -> None:
    if not update_lock.acquire(blocking=False):
        log(f"skip delivery={delivery} tag={tag}: update already running")
        return
    try:
        log(f"update start delivery={delivery} tag={tag}")
        result = subprocess.run(
            [
                str(UPDATE_SCRIPT),
                "--watch",
                "--release-tag",
                tag,
                "--allow-dirty",
                "--preserve-local-edits",
            ],
            cwd=str(CODEX_ROOT),
            text=True,
            capture_output=True,
            check=False,
        )
        if result.stdout:
            log(result.stdout.rstrip())
        if result.stderr:
            log(result.stderr.rstrip())
        log(f"update exit delivery={delivery} tag={tag} code={result.returncode}")
    finally:
        update_lock.release()


class Handler(BaseHTTPRequestHandler):
    server_version = "CodexReleaseWebhook/1.0"

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

        if not verify_signature(body, signature):
            log(f"reject delivery={delivery}: bad signature")
            self.send_error(401)
            return

        if event == "ping":
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"pong\n")
            log(f"ping delivery={delivery}")
            return

        if event != "release":
            self.send_response(202)
            self.end_headers()
            self.wfile.write(b"ignored\n")
            log(f"ignore delivery={delivery}: event={event}")
            return

        try:
            payload = json.loads(body.decode("utf-8"))
        except json.JSONDecodeError:
            self.send_error(400)
            return

        repo = payload.get("repository", {}).get("full_name", "")
        if EXPECTED_REPO and repo != EXPECTED_REPO:
            self.send_response(202)
            self.end_headers()
            self.wfile.write(b"wrong repo\n")
            log(f"ignore delivery={delivery}: repo={repo}")
            return

        action = payload.get("action", "")
        release = payload.get("release", {})
        tag = release.get("tag_name", "")
        draft = bool(release.get("draft"))
        if action not in {"created", "published", "released"} or draft or not tag:
            self.send_response(202)
            self.end_headers()
            self.wfile.write(b"ignored release action\n")
            log(f"ignore delivery={delivery}: action={action} draft={draft} tag={tag}")
            return

        threading.Thread(target=run_update, args=(tag, delivery), daemon=True).start()
        self.send_response(202)
        self.end_headers()
        self.wfile.write(b"update queued\n")

    def log_message(self, fmt: str, *args: object) -> None:
        return


if __name__ == "__main__":
    if not SECRET:
        raise SystemExit("CODEX_RELEASE_WEBHOOK_SECRET is required")
    if not UPDATE_SCRIPT.exists():
        raise SystemExit(f"missing updater: {UPDATE_SCRIPT}")
    server = ThreadingHTTPServer((HOST, PORT), Handler)
    log(f"listening host={HOST} port={PORT} path={WEBHOOK_PATH}")
    server.serve_forever()
