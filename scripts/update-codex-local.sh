#!/usr/bin/env bash
set -euo pipefail

CODEX_ROOT="${CODEX_ROOT:-/Users/williamxu/Desktop/Projects/codex}"
CODER_DIR="${CODEX_ROOT}/codex-rs"
LOCAL_BIN="${HOME}/.local/bin"
CURRENT_LINK_DIR="${LOCAL_BIN}"
LINK_TARGET="${LOCAL_BIN}/codex"
LINK_TARGET_TUI="${LOCAL_BIN}/codex-tui"
BACKUP_DIR="${HOME}/.local/lib/codex/releases"
STATE_DIR="${HOME}/.local/lib/codex"
STATE_FILE="${STATE_DIR}/release-tag.txt"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
NEW_TARGET_DIR="${BACKUP_DIR}/${TIMESTAMP}"
WATCH_MODE=0
FORCE=0

while [ $# -gt 0 ]; do
  case "${1:-}" in
    --watch)
      WATCH_MODE=1
      shift
      ;;
    --force)
      FORCE=1
      shift
      ;;
    -h|--help)
      cat <<'EOF'
Usage: update-codex-local.sh [--watch] [--force]

  --watch   Check latest upstream release before building; skip if unchanged.
  --force   Force a rebuild even if the watched release tag is unchanged.
  --help    Show this help text.
EOF
      exit 0
      ;;
    *)
      echo "unknown argument: $1"
      exit 1
      ;;
  esac
done

mkdir -p "$LOCAL_BIN" "$BACKUP_DIR" "$STATE_DIR"

if [ ! -d "$CODER_DIR" ]; then
  echo "Missing Codex source directory: $CODER_DIR"
  exit 1
fi

cd "$CODEX_ROOT"

extract_repo_from_remote() {
  local remote_url
  remote_url="$(git -C "$CODER_DIR" config --get remote.origin.url || true)"
  remote_url="${remote_url%.git}"
  case "$remote_url" in
    git@github.com:*) echo "${remote_url#git@github.com:}" ;;
    https://github.com/*) echo "${remote_url#https://github.com/}" ;;
    http://github.com/*) echo "${remote_url#http://github.com/}" ;;
    *) echo "" ;;
  esac
}

fetch_latest_release_tag() {
  local repo
  repo="$(extract_repo_from_remote || true)"
  if [ -n "$repo" ]; then
    local release_json
    if release_json="$(curl -fsSL "https://api.github.com/repos/${repo}/releases/latest" 2>/dev/null)"; then
      local tag
      tag="$(printf '%s\n' "$release_json" | sed -n 's/.*\"tag_name\"[[:space:]]*:[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p' | head -n 1)"
      if [ -n "$tag" ]; then
        echo "$tag"
        return 0
      fi
    fi
  fi

  git -C "$CODER_DIR" ls-remote --refs --tags --sort='-v:refname' origin \
    2>/dev/null \
    | sed -n 's#.*refs/tags/##p' \
    | sed 's/\^{}//' \
    | sed '/{}$/d' \
    | sed '/^$/d' \
    | head -n 1
}

if [ -n "$(git -C "$CODER_DIR" status --porcelain)" ]; then
  echo "Working tree is dirty in $CODER_DIR; aborting."
  git -C "$CODER_DIR" status --short
  exit 1
fi

if [ "$WATCH_MODE" -eq 1 ] && [ "$FORCE" -eq 0 ]; then
  remote_release="$(fetch_latest_release_tag || true)"
  if [ -z "$remote_release" ]; then
    echo "Could not determine remote release tag; skipping auto-update."
    exit 0
  fi
  current_release="$(cat "$STATE_FILE" 2>/dev/null || true)"
  if [ -n "$current_release" ] && [ "$current_release" = "$remote_release" ]; then
    echo "Release unchanged: $remote_release. Skipping update."
    exit 0
  fi
  echo "New release detected: ${current_release:-(none)} -> ${remote_release}. Rebuilding."
fi

git -C "$CODEX_ROOT" fetch --all --prune --tags
git -C "$CODER_DIR" pull --ff-only

if [ ! -d "$NEW_TARGET_DIR" ]; then
  mkdir -p "$NEW_TARGET_DIR"
fi

CARGO_TARGET_DIR="$NEW_TARGET_DIR" cargo build -p codex-cli -p codex-tui --manifest-path "$CODER_DIR/Cargo.toml"

NEW_CLI="${NEW_TARGET_DIR}/debug/codex"
NEW_TUI="${NEW_TARGET_DIR}/debug/codex-tui"
if [ ! -x "$NEW_CLI" ] || [ ! -x "$NEW_TUI" ]; then
  echo "Expected build artifacts not found:"
  echo "  $NEW_CLI"
  echo "  $NEW_TUI"
  exit 1
fi

TMP_LINK_CLI="$(mktemp "$CURRENT_LINK_DIR/.codex-cli.XXXXXX")"
TMP_LINK_TUI="$(mktemp "$CURRENT_LINK_DIR/.codex-tui.XXXXXX")"
trap 'rm -f "$TMP_LINK_CLI" "$TMP_LINK_TUI"' EXIT

ln -s "$NEW_CLI" "$TMP_LINK_CLI"
ln -s "$NEW_TUI" "$TMP_LINK_TUI"
mv -f "$TMP_LINK_CLI" "$LINK_TARGET"
mv -f "$TMP_LINK_TUI" "$LINK_TARGET_TUI"

if [ ! -x "$LINK_TARGET" ]; then
  echo "Failed to update $LINK_TARGET"
  exit 1
fi

ln -sfn "$NEW_TARGET_DIR" "${HOME}/.local/lib/codex/current"
hash -r
echo "Updated codex links:"
echo "  $(readlink -f "$LINK_TARGET")"
echo "  $(readlink -f "$LINK_TARGET_TUI")"
echo "Version:"
"$LINK_TARGET" --version

if [ "$WATCH_MODE" -eq 1 ] && [ -n "${remote_release:-}" ]; then
  echo "$remote_release" > "$STATE_FILE"
fi
