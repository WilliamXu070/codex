#!/usr/bin/env bash
set -euo pipefail

CODEX_ROOT="${CODEX_ROOT:-/Users/williamxu/Desktop/Projects/codex}"
UPDATE_SCRIPT="${CODEX_ROOT}/scripts/update-codex-local.sh"
LAUNCH_LABEL="com.williamxu.codex-release-watch"
LAUNCH_DIR="$HOME/Library/LaunchAgents"
PLIST_PATH="$LAUNCH_DIR/${LAUNCH_LABEL}.plist"
LOG_DIR="$HOME/Library/Logs/codex-release-watch"
LAUNCHD_RUNTIME="${HOME}/.local/bin/codex-release-watch-runner.sh"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-900}"
ACTION="install"
LAUNCHD_PATH="/bin/zsh"
LAUNCHD_CMD="source ${LAUNCHD_RUNTIME} --watch"
LAUNCHD_PATH_ENV="/Users/williamxu/.cargo/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"

usage() {
  cat <<'USAGE'
Usage: install-codex-release-watch.sh [install|uninstall|status] [--interval N]

  install   create and load launch agent (default)
  uninstall remove and unload launch agent
  status    show launch agent status

Examples:
  ./install-codex-release-watch.sh
  ./install-codex-release-watch.sh install --interval 900
  ./install-codex-release-watch.sh uninstall

Environment:
  CODEX_ROOT:      defaults to /Users/williamxu/Desktop/Projects/codex
  INTERVAL_SECONDS: polling interval in seconds
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
    --interval)
      if [[ -z "${2-}" ]]; then
        echo "--interval requires seconds" >&2
        exit 1
      fi
      INTERVAL_SECONDS="$2"
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

mkdir -p "$LOG_DIR" "$LAUNCH_DIR"

case "$ACTION" in
  install)
    if [[ ! -r "$UPDATE_SCRIPT" ]]; then
      echo "Missing readable updater: $UPDATE_SCRIPT" >&2
      exit 1
    fi
    mkdir -p "$(dirname "$LAUNCHD_RUNTIME")"
    cp "$UPDATE_SCRIPT" "$LAUNCHD_RUNTIME"
    chmod +x "$LAUNCHD_RUNTIME"

    cat > "$PLIST_PATH" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LAUNCH_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${LAUNCHD_PATH}</string>
    <string>-lc</string>
    <string>${LAUNCHD_CMD}</string>
  </array>
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
    <key>PATH</key>
    <string>${LAUNCHD_PATH_ENV}</string>
  </dict>
</dict>
</plist>
PLIST

    if launchctl list "${LAUNCH_LABEL}" >/dev/null 2>&1; then
      launchctl unload -w "$PLIST_PATH" || true
    fi
    launchctl load -w "$PLIST_PATH"
    echo "Installed and enabled launch agent: ${LAUNCH_LABEL}"
    echo "interval: ${INTERVAL_SECONDS}s"
    echo "plist: ${PLIST_PATH}"
    echo "update script: ${LAUNCHD_CMD}"
    ;;

  uninstall)
    if launchctl list "${LAUNCH_LABEL}" >/dev/null 2>&1; then
      launchctl unload -w "$PLIST_PATH" || true
    fi
    rm -f "$PLIST_PATH"
    rm -f "$LAUNCHD_RUNTIME"
    echo "Uninstalled launch agent: ${LAUNCH_LABEL}"
    ;;

  status)
    if launchctl list "${LAUNCH_LABEL}" >/dev/null 2>&1; then
      launchctl print "gui/$(id -u)/${LAUNCH_LABEL}" 2>/dev/null || launchctl list "$LAUNCH_LABEL"
    else
      echo "${LAUNCH_LABEL} is not loaded"
    fi
    if [[ -f "$PLIST_PATH" ]]; then
      echo "plist exists: yes"
    else
      echo "plist exists: no"
    fi
    echo "out log: ${LOG_DIR}/stdout.log"
    echo "err log: ${LOG_DIR}/stderr.log"
    ;;

  *)
    echo "Unknown action: ${ACTION}" >&2
    usage
    exit 1
    ;;
esac
