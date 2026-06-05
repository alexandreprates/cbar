#!/usr/bin/env bash
# cbar: Demonstrates dynamic status, simple thresholds, separators, and local diagnostics.
# deps: awk, date, df, uptime
# env: CBAR_SHOWCASE_DISK_WARN

set -euo pipefail

edit_cbar_env_item() {
  echo "Edit cbar env | bash=/bin/bash param1=-lc param2='mkdir -p \"\$HOME/.config/cbar\" && touch \"\$HOME/.config/cbar/env\" && if command -v cosmic-edit >/dev/null 2>&1; then cosmic-edit \"\$HOME/.config/cbar/env\" >/dev/null 2>&1 & elif command -v xdg-open >/dev/null 2>&1; then xdg-open \"\$HOME/.config/cbar/env\" >/dev/null 2>&1 & fi'"
}

disk_warn="${CBAR_SHOWCASE_DISK_WARN:-85}"
disk_percent="$(df -P / | awk 'NR == 2 { gsub("%", "", $5); print $5 }')"
load="$(awk '{ print $1 }' /proc/loadavg)"
status="OK"

if (( disk_percent >= disk_warn )); then
  status="Disk ${disk_percent}%"
fi

echo "${status}"
echo "---"
echo "System snapshot"
echo "--Load average: ${load} | disabled=true"
echo "--Root disk: ${disk_percent}% | disabled=true"
echo "--Disk warning threshold: ${disk_warn}% | disabled=true"
echo "Open root folder | shell=xdg-open param1=/"
echo "Open disk usage in terminal | shell=/bin/sh param1=-lc param2='df -h /; printf \"\\n\"; read -r -p \"Press enter to close...\"' terminal=true"
echo "---"
echo "Updated: $(date '+%Y-%m-%d %H:%M:%S') | disabled=true"
echo "---"
edit_cbar_env_item
