#!/usr/bin/env bash
# cbar: Demonstrates an inline SVG image as a RAM usage bar.
# deps: awk, base64, tr
# env: CBAR_SHOWCASE_RAM_WARN, CBAR_SHOWCASE_RAM_CRIT

set -euo pipefail

ram_warn="${CBAR_SHOWCASE_RAM_WARN:-70}"
ram_crit="${CBAR_SHOWCASE_RAM_CRIT:-90}"

read -r mem_total mem_available < <(
  awk '
    /^MemTotal:/ { total = $2 }
    /^MemAvailable:/ { available = $2 }
    END { print total, available }
  ' /proc/meminfo
)

used=$(( mem_total - mem_available ))
used_percent=$(( used * 100 / mem_total ))
fill_width=$(( used_percent * 12 / 100 ))
color="#2ea043"

if (( used_percent >= ram_crit )); then
  color="#f85149"
elif (( used_percent >= ram_warn )); then
  color="#d29922"
fi

svg_to_base64() {
  base64 | tr -d '\n'
}

bar_image="$(
  cat <<SVG | svg_to_base64
<svg xmlns="http://www.w3.org/2000/svg" width="16" height="18" viewBox="0 0 16 18">
  <rect width="16" height="18" rx="3" fill="#161b22"/>
  <rect x="2" y="4" width="${fill_width}" height="10" rx="2" fill="${color}"/>
</svg>
SVG
)"

echo "| image=${bar_image}"
echo "---"
echo "Memory usage"
echo "--Used: $(( used / 1024 )) MiB | disabled=true"
echo "--Available: $(( mem_available / 1024 )) MiB | disabled=true"
echo "--Warning threshold: ${ram_warn}% | disabled=true"
echo "--Critical threshold: ${ram_crit}% | disabled=true"
echo "Open memory details | shell=/bin/sh param1=-lc param2='free -h; printf \"\\n\"; read -r -p \"Press enter to close...\"' terminal=true"
