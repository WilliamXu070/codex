#!/usr/bin/env bash
set -euo pipefail

CODEX_ROOT="${CODEX_ROOT:-/Users/williamxu/Desktop/Projects/codex}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_AGENT="${SCRIPT_DIR}/codex-release-agent.py"
SOURCE_TRIGGER="${SCRIPT_DIR}/update-codex-local.sh"
LAUNCH_LABEL="com.williamxu.codex-release-watch"
LAUNCH_DIR="${HOME}/Library/LaunchAgents"
PLIST_PATH="${LAUNCH_DIR}/${LAUNCH_LABEL}.plist"
LOG_DIR="${HOME}/Library/Logs/codex-release-watch"
RUNTIME_DIR="${HOME}/.local/lib/codex"
AGENT_RUNTIME="${RUNTIME_DIR}/codex-release-agent.py"
TRIGGER_RUNTIME="${HOME}/.local/bin/codex-release-watch-runner.sh"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-900}"
CHANNEL="${CODEX_RELEASE_CHANNEL:-all}"
ACTION="install"
LAUNCHD_PATH_ENV="/Users/williamxu/.cargo/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"

usage() {
  cat <<'EOF'
Usage: install-codex-release-watch.sh [install|uninstall|status] [options]

  install                 Install the lightweight GitHub release check.
  uninstall               Unload and remove it.
  status                  Show launchd and release-ledger status.
  --interval SECONDS      Check interval (default: 900).
  --channel CHANNEL       all, stable, or prerelease (default: all).

The scheduled check does not invoke Codex for known tags. The SQLite release
ledger allows exactly one Codex agent attempt per newly discovered tag.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    install|uninstall|status)
      ACTION="$1"
      shift
      ;;
    --interval)
      INTERVAL_SECONDS="${2:-}"
      [[ "$INTERVAL_SECONDS" =~ ^[0-9]+$ ]] || {
        echo "--interval requires integer seconds" >&2
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
    [[ -r "$SOURCE_AGENT" ]] || {
      echo "missing release agent: $SOURCE_AGENT" >&2
      exit 1
    }
    [[ -r "$SOURCE_TRIGGER" ]] || {
      echo "missing release trigger: $SOURCE_TRIGGER" >&2
      exit 1
    }
    mkdir -p "$LAUNCH_DIR" "$LOG_DIR" "$RUNTIME_DIR" "$(dirname "$TRIGGER_RUNTIME")"
    cp "$SOURCE_AGENT" "$AGENT_RUNTIME"
    cp "$SOURCE_TRIGGER" "$TRIGGER_RUNTIME"
    chmod +x "$AGENT_RUNTIME" "$TRIGGER_RUNTIME"

    cat > "$PLIST_PATH" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LAUNCH_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${TRIGGER_RUNTIME}</string>
    <string>--watch</string>
    <string>--channel</string>
    <string>${CHANNEL}</string>
    <string>--delivery</string>
    <string>launchd-release-check</string>
  </array>
  <key>WorkingDirectory</key>
  <string>/tmp</string>
  <key>StartInterval</key>
  <integer>${INTERVAL_SECONDS}</integer>
  <key>RunAtLoad</key>
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
    <key>PATH</key>
    <string>${LAUNCHD_PATH_ENV}</string>
  </dict>
</dict>
</plist>
PLIST

    if launchctl list "$LAUNCH_LABEL" >/dev/null 2>&1; then
      launchctl unload -w "$PLIST_PATH" || true
    fi
    launchctl load -w "$PLIST_PATH"
    echo "installed: $LAUNCH_LABEL"
    echo "channel: $CHANNEL"
    echo "interval: ${INTERVAL_SECONDS}s"
    echo "agent ledger: ${RUNTIME_DIR}/release-agent/state.sqlite3"
    ;;
  uninstall)
    if launchctl list "$LAUNCH_LABEL" >/dev/null 2>&1; then
      launchctl unload -w "$PLIST_PATH" || true
    fi
    rm -f "$PLIST_PATH" "$TRIGGER_RUNTIME" "$AGENT_RUNTIME"
    echo "uninstalled: $LAUNCH_LABEL"
    ;;
  status)
    if launchctl list "$LAUNCH_LABEL" >/dev/null 2>&1; then
      launchctl print "gui/$(id -u)/${LAUNCH_LABEL}" 2>/dev/null ||
        launchctl list "$LAUNCH_LABEL"
    else
      echo "$LAUNCH_LABEL is not loaded"
    fi
    echo "out log: ${LOG_DIR}/stdout.log"
    echo "err log: ${LOG_DIR}/stderr.log"
    echo "agent ledger: ${RUNTIME_DIR}/release-agent/state.sqlite3"
    ;;
esac
