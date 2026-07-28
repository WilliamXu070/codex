#!/usr/bin/env bash
set -euo pipefail

CODEX_ROOT="${CODEX_ROOT:-/Users/williamxu/Desktop/Projects/codex}"
AGENT_SCRIPT="${CODEX_RELEASE_AGENT_SCRIPT:-${CODEX_ROOT}/scripts/codex-release-agent.py}"
CHANNEL="${CODEX_RELEASE_CHANNEL:-all}"
RELEASE_TAG=""
DELIVERY="manual"
RETRY_FAILED=0
NO_PUBLISH=0
NO_ACTIVATE=0

usage() {
  cat <<'EOF'
Usage: update-codex-local.sh [--watch] [--release-tag TAG] [options]

  --watch              Discover the newest official OpenAI Codex release.
  --release-tag TAG    Integrate one exact official release tag.
  --channel CHANNEL    all, stable, or prerelease (default: all).
  --delivery ID        GitHub delivery or manual request identifier.
  --retry-failed       Explicitly retry a previously failed tag.
  --no-publish         Validate without pushing, merging, or activating.
  --no-activate        Push and merge without replacing the active CLI.

The durable release ledger prevents polling and duplicate webhook deliveries
from launching Codex more than once for the same tag.
EOF
}

WATCH_MODE=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --watch)
      WATCH_MODE=1
      shift
      ;;
    --release-tag)
      RELEASE_TAG="${2:-}"
      [[ -n "$RELEASE_TAG" ]] || {
        echo "--release-tag requires a tag" >&2
        exit 1
      }
      shift 2
      ;;
    --channel)
      CHANNEL="${2:-}"
      [[ -n "$CHANNEL" ]] || {
        echo "--channel requires a value" >&2
        exit 1
      }
      shift 2
      ;;
    --delivery)
      DELIVERY="${2:-}"
      [[ -n "$DELIVERY" ]] || {
        echo "--delivery requires a value" >&2
        exit 1
      }
      shift 2
      ;;
    --retry-failed|--force)
      RETRY_FAILED=1
      shift
      ;;
    --no-publish)
      NO_PUBLISH=1
      shift
      ;;
    --no-activate)
      NO_ACTIVATE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ ! -x "$AGENT_SCRIPT" ]]; then
  echo "missing executable release agent: $AGENT_SCRIPT" >&2
  exit 1
fi
if [[ "$WATCH_MODE" -eq 0 && -z "$RELEASE_TAG" ]]; then
  echo "choose --watch or --release-tag TAG" >&2
  exit 1
fi
if [[ "$WATCH_MODE" -eq 1 && -n "$RELEASE_TAG" ]]; then
  echo "--watch and --release-tag are mutually exclusive" >&2
  exit 1
fi

args=(
  --repository openai/codex
  --source-root "$CODEX_ROOT"
  --delivery "$DELIVERY"
  --channel "$CHANNEL"
)
if [[ "$WATCH_MODE" -eq 1 ]]; then
  args+=(--latest)
else
  args+=(--release-tag "$RELEASE_TAG")
fi
if [[ "$RETRY_FAILED" -eq 1 ]]; then
  args+=(--retry-failed)
fi
if [[ "$NO_PUBLISH" -eq 1 ]]; then
  args+=(--no-publish)
fi
if [[ "$NO_ACTIVATE" -eq 1 ]]; then
  args+=(--no-activate)
fi

exec "$AGENT_SCRIPT" "${args[@]}"
