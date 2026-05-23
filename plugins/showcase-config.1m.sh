#!/usr/bin/env bash
# cbar: Demonstrates environment variables and user-tunable plugin behavior.
# deps: date
# env: CBAR_SHOWCASE_NAME, CBAR_SHOWCASE_URL, CBAR_SHOWCASE_MODE

set -euo pipefail

name="${CBAR_SHOWCASE_NAME:-cbar}"
url="${CBAR_SHOWCASE_URL:-https://github.com/alexandreprates/cbar}"
mode="${CBAR_SHOWCASE_MODE:-normal}"
updated="$(date '+%H:%M')"
script_path="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)/$(basename -- "${BASH_SOURCE[0]}")"

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
echo "Try focus mode | shell=/bin/sh param1=-lc param2='CBAR_SHOWCASE_MODE=focus \"${script_path}\"' terminal=true"
echo "---"
echo "Tip: export variables before launching cbar to tune plugins. | disabled=true"
