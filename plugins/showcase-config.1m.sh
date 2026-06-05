#!/usr/bin/env bash
# cbar: Demonstrates environment variables and user-tunable plugin behavior.
# deps: date
# env: CBAR_SHOWCASE_NAME, CBAR_SHOWCASE_URL, CBAR_SHOWCASE_MODE

set -euo pipefail

edit_cbar_env_item() {
  echo "Edit cbar env | bash=/bin/bash param1=-lc param2='mkdir -p \"\$HOME/.config/cbar\" && touch \"\$HOME/.config/cbar/env\" && if command -v cosmic-edit >/dev/null 2>&1; then cosmic-edit \"\$HOME/.config/cbar/env\" >/dev/null 2>&1 & elif command -v xdg-open >/dev/null 2>&1; then xdg-open \"\$HOME/.config/cbar/env\" >/dev/null 2>&1 & fi'"
}

name="${CBAR_SHOWCASE_NAME:-cbar}"
url="${CBAR_SHOWCASE_URL:-https://github.com/alexandreprates/cbar}"
mode="${CBAR_SHOWCASE_MODE:-normal}"
updated="$(date '+%H:%M')"

case "${mode}" in
  quiet)
    title="${name} quiet"
    ;;
  focus)
    title="${name} focus"
    ;;
  *)
    title="${name} ${updated}"
    ;;
esac

echo "${title}"
echo "---"
echo "Configuration"
echo "--CBAR_SHOWCASE_NAME=${name} | disabled=true"
echo "--CBAR_SHOWCASE_MODE=${mode} | disabled=true"
echo "--CBAR_SHOWCASE_URL=${url} | disabled=true"
echo "Open configured URL | href=${url}"
echo "---"
edit_cbar_env_item
echo "Tip: update ~/.config/cbar/env to tune plugins. | disabled=true"
