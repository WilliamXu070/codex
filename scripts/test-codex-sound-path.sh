#!/usr/bin/env bash
set -euo pipefail

CODEX_ROOT="${CODEX_ROOT:-/Users/williamxu/Desktop/Projects/codex}"
SOUND_CMD="${CODEX_ROOT}/william/commands/sound"
SOUND_HOOK="${CODEX_ROOT}/william/audio/random-sound"
STATE_FILE="$(mktemp "${TMPDIR:-/tmp}/codex-sound-state.XXXXXX")"
LOG_FILE="$(mktemp "${TMPDIR:-/tmp}/codex-sound-log.XXXXXX")"
trap 'rm -f "$STATE_FILE" "$LOG_FILE"' EXIT

if [ ! -x "$SOUND_CMD" ]; then
  echo "missing sound command: $SOUND_CMD" >&2
  exit 1
fi

if [ ! -x "$SOUND_HOOK" ]; then
  echo "missing sound hook: $SOUND_HOOK" >&2
  exit 1
fi

cat > "$STATE_FILE" <<'STATE'
typeset -g CODEX_SOUND_ENABLED=1
typeset -g CODEX_SOUND_VOLUME=1.00
typeset -g CODEX_SOUND_PROFILE=normal
typeset -g CODEX_SOUND_COMPLETION_DELAY=0
typeset -g CODEX_SOUND_COMPLETION_FILE=''
typeset -g CODEX_SOUND_APPROVAL_FILE=''
typeset -g CODEX_SOUND_DIR=''
STATE

env CODEX_SOUND_STATE_FILE="$STATE_FILE" "$SOUND_CMD" track set 06-kanye-poopity-scoop.mp3 >/dev/null
env CODEX_SOUND_STATE_FILE="$STATE_FILE" CODEX_SOUND_LOG="$LOG_FILE" CODEX_SOUND_DRY_RUN=1 \
  /bin/zsh "$SOUND_HOOK" --event completion '{"source":"sound-regression-completion"}'

if ! grep -q 'kind=completion' "$LOG_FILE" ||
   ! grep -q '/sounds/06-kanye-poopity-scoop.mp3' "$LOG_FILE"; then
  echo "completion sound selection regression" >&2
  cat "$LOG_FILE" >&2
  exit 1
fi

env CODEX_SOUND_STATE_FILE="$STATE_FILE" "$SOUND_CMD" approval set wilhelm-scream.mp3 >/dev/null
env CODEX_SOUND_STATE_FILE="$STATE_FILE" CODEX_SOUND_LOG="$LOG_FILE" CODEX_SOUND_DRY_RUN=1 \
  /bin/zsh "$SOUND_HOOK" --event approval-requested '{"source":"sound-regression-approval"}'

if ! grep -q 'kind=immediate' "$LOG_FILE" ||
   ! grep -q '/audio/wilhelm-scream.mp3' "$LOG_FILE"; then
  echo "approval sound selection regression" >&2
  cat "$LOG_FILE" >&2
  exit 1
fi

echo "codex sound path regression passed"
