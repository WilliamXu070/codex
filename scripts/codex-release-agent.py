#!/usr/bin/env python3
"""Run one idempotent Codex integration agent for an upstream release."""

from __future__ import annotations

import argparse
import contextlib
import dataclasses
import datetime as dt
import fcntl
import hashlib
import json
import os
import re
import shlex
import shutil
import sqlite3
import subprocess
import sys
import textwrap
import tomllib
import urllib.request
from pathlib import Path
from typing import IO


DEFAULT_SOURCE_ROOT = Path("/Users/williamxu/Desktop/Projects/codex")
DEFAULT_STATE_DIR = Path.home() / ".local/lib/codex/release-agent"
DEFAULT_UPSTREAM_URL = "https://github.com/openai/codex.git"
DEFAULT_REPOSITORY = "openai/codex"
TAG_RE = re.compile(
    r"^rust-v(?P<version>\d+\.\d+\.\d+(?:-(?:alpha|beta|rc)\.\d+)?)$"
)


class ReleaseAgentError(RuntimeError):
    pass


@dataclasses.dataclass(frozen=True)
class Claim:
    acquired: bool
    status: str
    attempts: int


@dataclasses.dataclass(frozen=True)
class RunResult:
    repository: str
    tag: str
    status: str
    branch: str | None = None
    workspace: str | None = None
    pr_url: str | None = None
    reason: str | None = None


class ReleaseLedger:
    def __init__(self, path: Path):
        self.path = path
        self.path.parent.mkdir(parents=True, exist_ok=True)
        with self.connect() as conn:
            conn.execute(
                """
                CREATE TABLE IF NOT EXISTS release_runs (
                    repository TEXT NOT NULL,
                    tag TEXT NOT NULL,
                    status TEXT NOT NULL,
                    attempts INTEGER NOT NULL,
                    delivery TEXT,
                    started_at TEXT NOT NULL,
                    finished_at TEXT,
                    branch TEXT,
                    workspace TEXT,
                    pr_url TEXT,
                    error TEXT,
                    PRIMARY KEY (repository, tag)
                )
                """
            )

    def connect(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self.path, timeout=30)
        conn.row_factory = sqlite3.Row
        return conn

    def claim(
        self,
        repository: str,
        tag: str,
        delivery: str,
        *,
        retry_failed: bool,
    ) -> Claim:
        now = utc_now()
        with self.connect() as conn:
            conn.execute("BEGIN IMMEDIATE")
            row = conn.execute(
                """
                SELECT status, attempts
                FROM release_runs
                WHERE repository = ? AND tag = ?
                """,
                (repository, tag),
            ).fetchone()
            if row is None:
                conn.execute(
                    """
                    INSERT INTO release_runs (
                        repository, tag, status, attempts, delivery, started_at
                    ) VALUES (?, ?, 'running', 1, ?, ?)
                    """,
                    (repository, tag, delivery, now),
                )
                return Claim(True, "running", 1)

            status = str(row["status"])
            attempts = int(row["attempts"])
            if status == "failed" and retry_failed:
                attempts += 1
                conn.execute(
                    """
                    UPDATE release_runs
                    SET status = 'running',
                        attempts = ?,
                        delivery = ?,
                        started_at = ?,
                        finished_at = NULL,
                        error = NULL
                    WHERE repository = ? AND tag = ?
                    """,
                    (attempts, delivery, now, repository, tag),
                )
                return Claim(True, "running", attempts)
            return Claim(False, status, attempts)

    def complete(
        self,
        repository: str,
        tag: str,
        *,
        branch: str,
        workspace: Path,
        pr_url: str | None,
    ) -> None:
        with self.connect() as conn:
            conn.execute(
                """
                UPDATE release_runs
                SET status = 'succeeded',
                    finished_at = ?,
                    branch = ?,
                    workspace = ?,
                    pr_url = ?,
                    error = NULL
                WHERE repository = ? AND tag = ?
                """,
                (
                    utc_now(),
                    branch,
                    str(workspace),
                    pr_url,
                    repository,
                    tag,
                ),
            )

    def fail(
        self,
        repository: str,
        tag: str,
        *,
        error: str,
        branch: str | None = None,
        workspace: Path | None = None,
    ) -> None:
        with self.connect() as conn:
            conn.execute(
                """
                UPDATE release_runs
                SET status = 'failed',
                    finished_at = ?,
                    branch = COALESCE(?, branch),
                    workspace = COALESCE(?, workspace),
                    error = ?
                WHERE repository = ? AND tag = ?
                """,
                (
                    utc_now(),
                    branch,
                    str(workspace) if workspace else None,
                    error[-8000:],
                    repository,
                    tag,
                ),
            )

    def get(self, repository: str, tag: str) -> dict[str, object] | None:
        with self.connect() as conn:
            row = conn.execute(
                """
                SELECT *
                FROM release_runs
                WHERE repository = ? AND tag = ?
                """,
                (repository, tag),
            ).fetchone()
            return dict(row) if row is not None else None


def utc_now() -> str:
    return dt.datetime.now(dt.UTC).isoformat()


def run_command(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
    stdout: IO[str] | int | None = subprocess.PIPE,
    stderr: IO[str] | int | None = subprocess.PIPE,
    timeout: int | None = None,
) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=stdout,
        stderr=stderr,
        check=False,
        timeout=timeout,
    )
    if completed.returncode != 0:
        stdout_text = completed.stdout if isinstance(completed.stdout, str) else ""
        stderr_text = completed.stderr if isinstance(completed.stderr, str) else ""
        detail = "\n".join(part for part in (stdout_text, stderr_text) if part).strip()
        rendered = shlex.join(command)
        raise ReleaseAgentError(
            f"command failed ({completed.returncode}): {rendered}"
            + (f"\n{detail}" if detail else "")
        )
    return completed


def git_output(root: Path, *args: str) -> str:
    return run_command(["git", *args], cwd=root).stdout.strip()


def validate_release(repository: str, tag: str) -> str:
    if repository != DEFAULT_REPOSITORY:
        raise ReleaseAgentError(
            f"refusing release repository {repository!r}; expected {DEFAULT_REPOSITORY!r}"
        )
    match = TAG_RE.fullmatch(tag)
    if match is None:
        raise ReleaseAgentError(f"unsupported Codex release tag: {tag!r}")
    return match.group("version")


def discover_latest_release(repository: str, channel: str) -> str:
    if repository != DEFAULT_REPOSITORY:
        raise ReleaseAgentError(
            f"refusing release repository {repository!r}; expected {DEFAULT_REPOSITORY!r}"
        )
    request = urllib.request.Request(
        f"https://api.github.com/repos/{repository}/releases?per_page=50",
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "codex-release-agent/1.0",
        },
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        releases = json.load(response)
    for release in releases:
        if release.get("draft"):
            continue
        is_prerelease = bool(release.get("prerelease"))
        if channel == "stable" and is_prerelease:
            continue
        if channel == "prerelease" and not is_prerelease:
            continue
        tag = str(release.get("tag_name", ""))
        if TAG_RE.fullmatch(tag):
            return tag
    raise ReleaseAgentError(
        f"no {channel} Codex release found for {repository}"
    )


def safe_tag_name(tag: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "-", tag).strip("-")


def parse_github_repository(remote_url: str) -> str:
    normalized = remote_url.removesuffix(".git")
    if normalized.startswith("git@github.com:"):
        return normalized.split(":", 1)[1]
    marker = "github.com/"
    if marker in normalized:
        return normalized.split(marker, 1)[1]
    raise ReleaseAgentError(f"origin is not a GitHub repository: {remote_url}")


def hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def untracked_paths(source_root: Path) -> list[Path]:
    raw = subprocess.check_output(
        ["git", "ls-files", "--others", "--exclude-standard", "-z"],
        cwd=source_root,
    )
    paths: list[Path] = []
    for item in raw.split(b"\0"):
        if not item:
            continue
        relative = Path(os.fsdecode(item))
        absolute = (source_root / relative).resolve()
        try:
            absolute.relative_to(source_root.resolve())
        except ValueError as exc:
            raise ReleaseAgentError(f"unsafe untracked path: {relative}") from exc
        paths.append(relative)
    return sorted(paths, key=lambda path: str(path))


def source_fingerprint(source_root: Path) -> str:
    digest = hashlib.sha256()
    digest.update(git_output(source_root, "rev-parse", "HEAD").encode())
    digest.update(
        subprocess.check_output(
            ["git", "status", "--porcelain=v1", "-z"],
            cwd=source_root,
        )
    )
    digest.update(
        subprocess.check_output(
            ["git", "diff", "--binary", "HEAD", "--"],
            cwd=source_root,
        )
    )
    for relative in untracked_paths(source_root):
        absolute = source_root / relative
        digest.update(os.fsencode(str(relative)))
        if absolute.is_symlink():
            digest.update(os.fsencode(os.readlink(absolute)))
        elif absolute.is_file():
            digest.update(hash_file(absolute).encode())
    return digest.hexdigest()


def write_release_context(source_root: Path, workspace: Path) -> Path:
    context_dir = workspace / ".codex-release-context"
    if context_dir.exists():
        shutil.rmtree(context_dir)
    untracked_dir = context_dir / "untracked"
    untracked_dir.mkdir(parents=True)

    patch = subprocess.check_output(
        ["git", "diff", "--binary", "HEAD", "--"],
        cwd=source_root,
    )
    (context_dir / "dirty.patch").write_bytes(patch)
    (context_dir / "status.txt").write_text(
        git_output(source_root, "status", "--short", "--branch") + "\n",
        encoding="utf-8",
    )

    manifest: list[dict[str, str]] = []
    for relative in untracked_paths(source_root):
        source = source_root / relative
        destination = untracked_dir / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        if source.is_symlink():
            destination.symlink_to(os.readlink(source))
            kind = "symlink"
        elif source.is_file():
            shutil.copy2(source, destination)
            kind = "file"
        else:
            continue
        manifest.append({"path": str(relative), "kind": kind})
    (context_dir / "untracked.json").write_text(
        json.dumps(manifest, indent=2) + "\n",
        encoding="utf-8",
    )

    exclude_file = workspace / ".git/info/exclude"
    with exclude_file.open("a", encoding="utf-8") as handle:
        handle.write("\n/.codex-release-context/\n")
    return context_dir


def prepare_workspace(
    *,
    source_root: Path,
    state_dir: Path,
    tag: str,
    retry_failed: bool,
    upstream_url: str,
) -> tuple[Path, str, str, Path]:
    safe_tag = safe_tag_name(tag)
    workspace = state_dir / "workspaces" / safe_tag
    branch = f"agent/upstream-{tag.removeprefix('rust-v')}"
    origin_url = git_output(source_root, "remote", "get-url", "origin")
    source_head = git_output(source_root, "rev-parse", "HEAD")

    if workspace.exists() and not retry_failed:
        raise ReleaseAgentError(f"release workspace already exists: {workspace}")
    if not workspace.exists():
        workspace.parent.mkdir(parents=True, exist_ok=True)
        run_command(
            ["git", "clone", "--no-local", str(source_root), str(workspace)],
            cwd=source_root,
        )
        run_command(
            ["git", "remote", "set-url", "origin", origin_url],
            cwd=workspace,
        )
        run_command(["git", "switch", "-c", branch, source_head], cwd=workspace)
    elif git_output(workspace, "branch", "--show-current") != branch:
        raise ReleaseAgentError(
            f"retry workspace is on unexpected branch: {workspace}"
        )

    remotes = set(git_output(workspace, "remote").splitlines())
    if "upstream" not in remotes:
        run_command(["git", "remote", "add", "upstream", upstream_url], cwd=workspace)
    else:
        run_command(
            ["git", "remote", "set-url", "upstream", upstream_url],
            cwd=workspace,
        )

    run_command(["git", "fetch", "--prune", "origin", "main"], cwd=workspace)
    run_command(
        [
            "git",
            "fetch",
            "--force",
            "upstream",
            f"refs/tags/{tag}:refs/tags/{tag}",
        ],
        cwd=workspace,
    )
    context_dir = write_release_context(source_root, workspace)
    return workspace, branch, source_head, context_dir


def integration_prompt(
    *,
    source_root: Path,
    workspace: Path,
    context_dir: Path,
    tag: str,
    version: str,
    source_head: str,
    branch: str,
) -> str:
    return textwrap.dedent(
        f"""
        You are the unattended Codex upstream-integration agent for William's custom
        Codex fork. Work only inside this isolated clone:

          {workspace}

        The live checkout at {source_root} is dirty and read-only for this task.
        Never edit, stash, reset, clean, commit, or switch that live checkout.

        Goal: produce one clean commit history on branch {branch} that contains both
        the personal fork and official OpenAI Codex release {tag} ({version}).

        Required workflow:

        1. Inspect the current branch, origin/main, {tag}, and the release context in
           {context_dir}. The clone began at live source commit {source_head}.
        2. Merge origin/main into this branch, resolving divergence without dropping
           either side's intentional changes.
        3. Merge the exact official tag {tag}. Resolve conflicts semantically: keep
           the newer upstream architecture while reapplying William's custom behavior.
           Do not merely change the Cargo version.
        4. Port every intentional dirty tracked edit from
           {context_dir / 'dirty.patch'}. Use `git apply --3way` as a starting point
           when useful, then resolve manually.
        5. Inspect {context_dir / 'untracked.json'} and port intentional untracked
           source/scripts. Exclude generated `*.snap.new` files and the accidental
           `.bazelversion 2`. Preserve real Rust source, regression tests, updater
           scripts, sound/transcription helpers, and documentation.
        6. Preserve and verify the custom `/sound` routing, random completion and
           approval sounds, transcription capture/RMS/waveform behavior, clipboard
           repair/copy work, and their helper scripts. Adapt them to upstream APIs
           when source structure changed.
        7. Ensure `codex-rs/Cargo.toml` reports workspace version {version}. Run
           `cargo fmt --all` from `codex-rs`, the focused custom tests you can
           identify, and `CODEX_ROOT={workspace} bash scripts/test-codex-sound-path.sh`.
        8. Commit all intended integration changes. Leave the worktree clean.

        Do not push, open a PR, alter git remotes, update the live binary symlink, or
        delete release context. The orchestrator independently validates and
        publishes only after you finish.
        """
    ).strip()


def run_codex_agent(
    *,
    workspace: Path,
    prompt: str,
    state_dir: Path,
    tag: str,
    codex_binary: str,
    timeout_seconds: int,
) -> None:
    log_dir = state_dir / "logs" / safe_tag_name(tag)
    log_dir.mkdir(parents=True, exist_ok=True)
    jsonl_path = log_dir / "codex-events.jsonl"
    stderr_path = log_dir / "codex-stderr.log"
    last_message_path = log_dir / "last-message.md"
    command = [
        codex_binary,
        "exec",
        "--ephemeral",
        "--json",
        "-c",
        'default_permissions="release_agent"',
        "-c",
        (
            'permissions.release_agent.filesystem={'
            '":minimal"="read",'
            '":tmpdir"="write",'
            '":slash_tmp"="write",'
            '":workspace_roots"={"."="write",".git"="write"}'
            "}"
        ),
        "-c",
        'approval_policy="never"',
        "-C",
        str(workspace),
        "-o",
        str(last_message_path),
        prompt,
    ]
    with (
        jsonl_path.open("w", encoding="utf-8") as stdout_handle,
        stderr_path.open("w", encoding="utf-8") as stderr_handle,
    ):
        run_command(
            command,
            cwd=workspace,
            stdout=stdout_handle,
            stderr=stderr_handle,
            timeout=timeout_seconds,
        )


def workspace_version(workspace: Path) -> str:
    manifest = workspace / "codex-rs/Cargo.toml"
    with manifest.open("rb") as handle:
        data = tomllib.load(handle)
    return str(data["workspace"]["package"]["version"])


def verify_workspace(
    *,
    workspace: Path,
    source_root: Path,
    source_fingerprint_before: str,
    tag: str,
    version: str,
    skip_build: bool,
) -> None:
    source_fingerprint_after = source_fingerprint(source_root)
    if source_fingerprint_after != source_fingerprint_before:
        raise ReleaseAgentError("the live dirty checkout changed during the agent run")

    if git_output(workspace, "status", "--porcelain"):
        raise ReleaseAgentError("agent left the integration workspace dirty")

    run_command(
        ["git", "merge-base", "--is-ancestor", "origin/main", "HEAD"],
        cwd=workspace,
    )
    run_command(
        ["git", "merge-base", "--is-ancestor", f"refs/tags/{tag}", "HEAD"],
        cwd=workspace,
    )
    actual_version = workspace_version(workspace)
    if actual_version != version:
        raise ReleaseAgentError(
            f"integrated workspace version is {actual_version}, expected {version}"
        )

    required_paths = [
        "scripts/test-codex-sound-path.sh",
        "william/audio/random-sound",
        "william/commands/sound",
        "william/transcribe/transcribe-command",
    ]
    missing = [path for path in required_paths if not (workspace / path).exists()]
    if missing:
        raise ReleaseAgentError(
            "custom Codex files are missing: " + ", ".join(missing)
        )

    test_env = os.environ.copy()
    test_env["CODEX_ROOT"] = str(workspace)
    run_command(
        ["bash", "scripts/test-codex-sound-path.sh"],
        cwd=workspace,
        env=test_env,
    )
    run_command(
        ["cargo", "fmt", "--all", "--", "--check"],
        cwd=workspace / "codex-rs",
    )
    if not skip_build:
        run_command(
            ["cargo", "build", "-p", "codex-cli", "-p", "codex-tui"],
            cwd=workspace / "codex-rs",
            timeout=7200,
        )
        built_codex = workspace / "codex-rs/target/debug/codex"
        output = run_command([str(built_codex), "--version"], cwd=workspace).stdout
        if version not in output:
            raise ReleaseAgentError(
                f"built Codex reports {output.strip()!r}, expected {version!r}"
            )


def publish_branch(
    *,
    workspace: Path,
    branch: str,
    tag: str,
    version: str,
) -> str:
    run_command(["git", "push", "-u", "origin", branch], cwd=workspace)
    origin_url = git_output(workspace, "remote", "get-url", "origin")
    repository = parse_github_repository(origin_url)
    existing = run_command(
        [
            "gh",
            "pr",
            "list",
            "--repo",
            repository,
            "--head",
            branch,
            "--state",
            "all",
            "--json",
            "url",
            "--jq",
            ".[0].url // empty",
        ],
        cwd=workspace,
    ).stdout.strip()
    if existing:
        return existing

    body_path = workspace / ".codex-release-pr-body.md"
    body_path.write_text(
        textwrap.dedent(
            f"""
            ## What changed

            Integrated official OpenAI Codex release `{tag}` into the custom fork,
            while preserving the local sound, transcription, clipboard, and updater
            behavior.

            ## Why

            The previous updater rebuilt a stale dirty checkout instead of fetching
            and integrating the detected upstream release.

            ## Validation

            - Both `origin/main` and `{tag}` are ancestors of this branch.
            - Workspace and built CLI version: `{version}`.
            - Custom sound-path regression passed.
            - `cargo fmt --all -- --check` passed.
            - `cargo build -p codex-cli -p codex-tui` passed.

            This PR was produced by one deduplicated release-agent run. Review before
            merging; it does not update the live binary automatically.
            """
        ).strip()
        + "\n",
        encoding="utf-8",
    )
    try:
        return run_command(
            [
                "gh",
                "pr",
                "create",
                "--repo",
                repository,
                "--base",
                "main",
                "--head",
                branch,
                "--draft",
                "--title",
                f"Integrate Codex {version}",
                "--body-file",
                str(body_path),
            ],
            cwd=workspace,
        ).stdout.strip()
    finally:
        body_path.unlink(missing_ok=True)


@contextlib.contextmanager
def exclusive_lock(path: Path):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+", encoding="utf-8") as handle:
        fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def execute(args: argparse.Namespace) -> RunResult:
    source_root = args.source_root.resolve()
    state_dir = args.state_dir.resolve()
    version = validate_release(args.repository, args.release_tag)
    ledger = ReleaseLedger(state_dir / "state.sqlite3")
    branch: str | None = None
    workspace: Path | None = None

    with exclusive_lock(state_dir / "agent.lock"):
        claim = ledger.claim(
            args.repository,
            args.release_tag,
            args.delivery,
            retry_failed=args.retry_failed,
        )
        if not claim.acquired:
            return RunResult(
                repository=args.repository,
                tag=args.release_tag,
                status="skipped",
                reason=f"already {claim.status} after {claim.attempts} attempt(s)",
            )

        try:
            before = source_fingerprint(source_root)
            workspace, branch, source_head, context_dir = prepare_workspace(
                source_root=source_root,
                state_dir=state_dir,
                tag=args.release_tag,
                retry_failed=args.retry_failed,
                upstream_url=args.upstream_url,
            )
            prompt = integration_prompt(
                source_root=source_root,
                workspace=workspace,
                context_dir=context_dir,
                tag=args.release_tag,
                version=version,
                source_head=source_head,
                branch=branch,
            )
            run_codex_agent(
                workspace=workspace,
                prompt=prompt,
                state_dir=state_dir,
                tag=args.release_tag,
                codex_binary=args.codex_binary,
                timeout_seconds=args.timeout_seconds,
            )
            verify_workspace(
                workspace=workspace,
                source_root=source_root,
                source_fingerprint_before=before,
                tag=args.release_tag,
                version=version,
                skip_build=args.skip_build,
            )
            pr_url = None
            if not args.no_publish:
                pr_url = publish_branch(
                    workspace=workspace,
                    branch=branch,
                    tag=args.release_tag,
                    version=version,
                )
            ledger.complete(
                args.repository,
                args.release_tag,
                branch=branch,
                workspace=workspace,
                pr_url=pr_url,
            )
            return RunResult(
                repository=args.repository,
                tag=args.release_tag,
                status="succeeded",
                branch=branch,
                workspace=str(workspace),
                pr_url=pr_url,
            )
        except Exception as exc:
            ledger.fail(
                args.repository,
                args.release_tag,
                error=str(exc),
                branch=branch,
                workspace=workspace,
            )
            raise


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Integrate one OpenAI Codex release through one deduplicated agent run."
    )
    release = parser.add_mutually_exclusive_group(required=True)
    release.add_argument("--release-tag")
    release.add_argument(
        "--latest",
        action="store_true",
        help="Discover the newest matching official GitHub release.",
    )
    parser.add_argument(
        "--channel",
        choices=("all", "stable", "prerelease"),
        default=os.environ.get("CODEX_RELEASE_CHANNEL", "all"),
    )
    parser.add_argument("--delivery", default="manual")
    parser.add_argument("--repository", default=DEFAULT_REPOSITORY)
    parser.add_argument(
        "--source-root",
        type=Path,
        default=Path(os.environ.get("CODEX_ROOT", DEFAULT_SOURCE_ROOT)),
    )
    parser.add_argument(
        "--state-dir",
        type=Path,
        default=Path(os.environ.get("CODEX_RELEASE_AGENT_STATE_DIR", DEFAULT_STATE_DIR)),
    )
    parser.add_argument(
        "--upstream-url",
        default=os.environ.get("CODEX_UPSTREAM_URL", DEFAULT_UPSTREAM_URL),
    )
    parser.add_argument(
        "--codex-binary",
        default=os.environ.get("CODEX_RELEASE_AGENT_BINARY", "codex"),
    )
    parser.add_argument(
        "--timeout-seconds",
        type=int,
        default=int(os.environ.get("CODEX_RELEASE_AGENT_TIMEOUT", "7200")),
    )
    parser.add_argument(
        "--retry-failed",
        action="store_true",
        help="Manually retry a tag whose prior run is recorded as failed.",
    )
    parser.add_argument(
        "--no-publish",
        action="store_true",
        help="Validate the integration without pushing or opening a PR.",
    )
    parser.add_argument(
        "--skip-build",
        action="store_true",
        help=argparse.SUPPRESS,
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        if args.latest:
            args.release_tag = discover_latest_release(args.repository, args.channel)
        result = execute(args)
    except Exception as exc:
        print(
            json.dumps(
                {
                    "repository": args.repository,
                    "tag": args.release_tag,
                    "status": "failed",
                    "error": str(exc),
                },
                sort_keys=True,
            )
        )
        return 1
    print(json.dumps(dataclasses.asdict(result), sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
