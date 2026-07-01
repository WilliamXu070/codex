#!/usr/bin/env bash
set -euo pipefail

CODEX_ROOT="${CODEX_ROOT:-/Users/williamxu/Desktop/Projects/codex}"
CODER_DIR="${CODEX_ROOT}/codex-rs"
LOCAL_BIN="${HOME}/.local/bin"
CURRENT_LINK_DIR="${LOCAL_BIN}"
LINK_TARGET="${LOCAL_BIN}/codex"
LINK_TARGET_TUI="${LOCAL_BIN}/codex-tui"
BACKUP_DIR="${HOME}/.local/lib/codex/releases"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
NEW_TARGET_DIR="${BACKUP_DIR}/${TIMESTAMP}"

mkdir -p "$LOCAL_BIN" "$BACKUP_DIR"

if [ ! -d "$CODER_DIR" ]; then
  echo "Missing Codex source directory: $CODER_DIR"
  exit 1
fi

if [ -n "$(git -C "$CODER_DIR" status --porcelain)" ]; then
  echo "Working tree is dirty in $CODER_DIR; aborting."
  git -C "$CODER_DIR" status --short
  exit 1
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
