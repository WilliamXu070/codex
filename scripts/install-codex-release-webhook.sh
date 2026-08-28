#!/usr/bin/env bash
set -euo pipefail

CODEX_ROOT="${CODEX_ROOT:-/Users/williamxu/Desktop/Projects/codex}"
SERVER_SCRIPT="${CODEX_ROOT}/scripts/codex-release-webhook-server.py"
LAUNCH_LABEL="com.williamxu.codex-release-webhook"
LAUNCH_DIR="$HOME/Library/LaunchAgents"
PLIST_PATH="$LAUNCH_DIR/${LAUNCH_LABEL}.plist"
LOG_DIR="$HOME/Library/Logs/codex-release-webhook"
RUNTIME_DIR="$HOME/.local/lib/codex"
RUNTIME_SCRIPT="${RUNTIME_DIR}/codex-release-webhook-server.py"
HOST="${CODEX_RELEASE_WEBHOOK_HOST:-127.0.0.1}"
PORT="${CODEX_RELEASE_WEBHOOK_PORT:-8765}"
SECRET="${CODEX_RELEASE_WEBHOOK_SECRET:-}"
WATCH_LABEL="com.williamxu.codex-release-watch"
WATCH_PLIST_PATH="$LAUNCH_DIR/${WATCH_LABEL}.plist"
WATCH_RUNTIME="/private/tmp/codex-release-watch-runner.sh"
ACTION="install"

usage() {
  cat <<'USAGE'
Usage: install-codex-release-webhook.sh [install|uninstall|status] [--secret SECRET] [--port PORT]

  install    create and load the local release webhook receiver
  uninstall  unload and remove the LaunchAgent
  status     show LaunchAgent status and log paths

GitHub webhook:
  payload URL: https://<your-public-tunnel>/github-release-webhook
  content type: application/json
  secret: same value passed here
  event: Releases
USAGE
}

if [[ "${1-}" == "--help" || "${1-}" == "-h" ]]; then
  usage
  exit 0
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    install|uninstall|status)
      ACTION="$1"
      ;;
    --secret)
      SECRET="${2:-}"
      if [[ -z "$SECRET" ]]; then
        echo "--secret requires a value" >&2
        exit 1
      fi
      shift
      ;;
    --port)
      PORT="${2:-}"
      if [[ -z "$PORT" ]]; then
        echo "--port requires a value" >&2
        exit 1
      fi
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
  shift
done

disable_polling_agent() {
  if launchctl list "$WATCH_LABEL" >/dev/null 2>&1; then
    launchctl unload -w "$WATCH_PLIST_PATH" || true
  fi
  rm -f "$WATCH_PLIST_PATH" "$WATCH_RUNTIME"
}

case "$ACTION" in
  install)
    if [[ -z "$SECRET" ]]; then
      echo "Set CODEX_RELEASE_WEBHOOK_SECRET or pass --secret." >&2
      exit 1
    fi
    if [[ ! -r "$SERVER_SCRIPT" ]]; then
      echo "Missing readable webhook server: $SERVER_SCRIPT" >&2
      exit 1
    fi

    mkdir -p "$LAUNCH_DIR" "$LOG_DIR" "$RUNTIME_DIR"
    cp "$SERVER_SCRIPT" "$RUNTIME_SCRIPT"
    chmod +x "$RUNTIME_SCRIPT"

    cat > "$PLIST_PATH" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LAUNCH_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/bin/python3</string>
    <string>${RUNTIME_SCRIPT}</string>
  </array>
  <key>WorkingDirectory</key>
  <string>/tmp</string>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>${LOG_DIR}/stdout.log</string>
  <key>StandardErrorPath</key>
  <string>${LOG_DIR}/stderr.log</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>CODEX_ROOT</key>
    <string>${CODEX_ROOT}</string>
    <key>CODEX_RELEASE_WEBHOOK_SECRET</key>
    <string>${SECRET}</string>
    <key>CODEX_RELEASE_WEBHOOK_HOST</key>
    <string>${HOST}</string>
    <key>CODEX_RELEASE_WEBHOOK_PORT</key>
    <string>${PORT}</string>
    <key>PATH</key>
    <string>/Users/williamxu/.cargo/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
  </dict>
</dict>
</plist>
PLIST

  if launchctl list "$LAUNCH_LABEL" >/dev/null 2>&1; then
      launchctl unload -w "$PLIST_PATH" || true
    fi
    disable_polling_agent
    launchctl load -w "$PLIST_PATH"
    echo "Installed and enabled launch agent: $LAUNCH_LABEL"
    echo "local URL: http://${HOST}:${PORT}/github-release-webhook"
    echo "logs: $LOG_DIR"
    ;;

  uninstall)
    if launchctl list "$LAUNCH_LABEL" >/dev/null 2>&1; then
      launchctl unload -w "$PLIST_PATH" || true
    fi
    rm -f "$PLIST_PATH"
    echo "Uninstalled launch agent: $LAUNCH_LABEL"
    ;;

  status)
    if launchctl list "$LAUNCH_LABEL" >/dev/null 2>&1; then
      launchctl print "gui/$(id -u)/${LAUNCH_LABEL}" 2>/dev/null || launchctl list "$LAUNCH_LABEL"
    else
      echo "$LAUNCH_LABEL is not loaded"
    fi
    [[ -f "$PLIST_PATH" ]] && echo "plist exists: yes" || echo "plist exists: no"
    echo "local URL: http://${HOST}:${PORT}/github-release-webhook"
    echo "out log: ${LOG_DIR}/stdout.log"
    echo "err log: ${LOG_DIR}/stderr.log"
    ;;

  *)
    echo "Unknown action: $ACTION" >&2
    usage
    exit 1
  ;;
esac
