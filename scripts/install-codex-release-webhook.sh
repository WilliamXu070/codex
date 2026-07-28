#!/usr/bin/env bash
set -euo pipefail

CODEX_ROOT="${CODEX_ROOT:-/Users/williamxu/Desktop/Projects/codex}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_SERVER="${SCRIPT_DIR}/codex-release-webhook-server.py"
SOURCE_AGENT="${SCRIPT_DIR}/codex-release-agent.py"
LAUNCH_LABEL="com.williamxu.codex-release-webhook"
LAUNCH_DIR="${HOME}/Library/LaunchAgents"
PLIST_PATH="${LAUNCH_DIR}/${LAUNCH_LABEL}.plist"
LOG_DIR="${HOME}/Library/Logs/codex-release-webhook"
RUNTIME_DIR="${HOME}/.local/lib/codex"
SERVER_RUNTIME="${RUNTIME_DIR}/codex-release-webhook-server.py"
AGENT_RUNTIME="${RUNTIME_DIR}/codex-release-agent.py"
HOST="${CODEX_RELEASE_WEBHOOK_HOST:-127.0.0.1}"
PORT="${CODEX_RELEASE_WEBHOOK_PORT:-8765}"
SECRET="${CODEX_RELEASE_WEBHOOK_SECRET:-}"
CHANNEL="${CODEX_RELEASE_CHANNEL:-all}"
PYTHON_BIN="${CODEX_RELEASE_PYTHON:-$(command -v python3)}"
ACTION="install"

usage() {
  cat <<'EOF'
Usage: install-codex-release-webhook.sh [install|uninstall|status] [options]

  --secret SECRET      GitHub webhook HMAC secret.
  --port PORT          Local listener port (default: 8765).
  --channel CHANNEL    all, stable, or prerelease.

The endpoint accepts only signed, newly published `openai/codex` release events.
Duplicate deliveries are deduplicated by the release-agent ledger.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    install|uninstall|status)
      ACTION="$1"
      shift
      ;;
    --secret)
      SECRET="${2:-}"
      [[ -n "$SECRET" ]] || {
        echo "--secret requires a value" >&2
        exit 1
      }
      shift 2
      ;;
    --port)
      PORT="${2:-}"
      [[ "$PORT" =~ ^[0-9]+$ ]] || {
        echo "--port requires an integer" >&2
        exit 1
      }
      shift 2
      ;;
    --channel)
      CHANNEL="${2:-}"
      [[ "$CHANNEL" == "all" || "$CHANNEL" == "stable" || "$CHANNEL" == "prerelease" ]] || {
        echo "--channel must be all, stable, or prerelease" >&2
        exit 1
      }
      shift 2
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

case "$ACTION" in
  install)
    [[ -n "$SECRET" ]] || {
      echo "set CODEX_RELEASE_WEBHOOK_SECRET or pass --secret" >&2
      exit 1
    }
    mkdir -p "$LAUNCH_DIR" "$LOG_DIR" "$RUNTIME_DIR"
    "$PYTHON_BIN" -c 'import tomllib' >/dev/null 2>&1 || {
      echo "release agent requires Python 3.11 or newer: $PYTHON_BIN" >&2
      exit 1
    }
    cp "$SOURCE_SERVER" "$SERVER_RUNTIME"
    cp "$SOURCE_AGENT" "$AGENT_RUNTIME"
    chmod +x "$SERVER_RUNTIME" "$AGENT_RUNTIME"

    cat > "$PLIST_PATH" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LAUNCH_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${PYTHON_BIN}</string>
    <string>${SERVER_RUNTIME}</string>
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
    <key>CODEX_RELEASE_AGENT_SCRIPT</key>
    <string>${AGENT_RUNTIME}</string>
    <key>CODEX_RELEASE_WEBHOOK_SECRET</key>
    <string>${SECRET}</string>
    <key>CODEX_RELEASE_WEBHOOK_HOST</key>
    <string>${HOST}</string>
    <key>CODEX_RELEASE_WEBHOOK_PORT</key>
    <string>${PORT}</string>
    <key>CODEX_RELEASE_WEBHOOK_REPO</key>
    <string>openai/codex</string>
    <key>CODEX_RELEASE_CHANNEL</key>
    <string>${CHANNEL}</string>
    <key>PATH</key>
    <string>/Users/williamxu/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
  </dict>
</dict>
</plist>
PLIST

    if launchctl list "$LAUNCH_LABEL" >/dev/null 2>&1; then
      launchctl unload -w "$PLIST_PATH" || true
    fi
    launchctl load -w "$PLIST_PATH"
    echo "installed: $LAUNCH_LABEL"
    echo "local URL: http://${HOST}:${PORT}/github-release-webhook"
    echo "repo: openai/codex"
    echo "channel: $CHANNEL"
    ;;
  uninstall)
    if launchctl list "$LAUNCH_LABEL" >/dev/null 2>&1; then
      launchctl unload -w "$PLIST_PATH" || true
    fi
    rm -f "$PLIST_PATH" "$SERVER_RUNTIME"
    echo "uninstalled: $LAUNCH_LABEL"
    ;;
  status)
    if launchctl list "$LAUNCH_LABEL" >/dev/null 2>&1; then
      launchctl print "gui/$(id -u)/${LAUNCH_LABEL}" 2>/dev/null ||
        launchctl list "$LAUNCH_LABEL"
    else
      echo "$LAUNCH_LABEL is not loaded"
    fi
    echo "local URL: http://${HOST}:${PORT}/github-release-webhook"
    echo "out log: ${LOG_DIR}/stdout.log"
    echo "err log: ${LOG_DIR}/stderr.log"
    ;;
esac
