#!/usr/bin/env bash
# cbar: Demonstrates an inline SVG image as a compact RAM usage gauge.
# deps: awk, base64, tr
# env: CBAR_SHOWCASE_RAM_WARN, CBAR_SHOWCASE_RAM_CRIT

set -euo pipefail

ram_warn="${CBAR_SHOWCASE_RAM_WARN:-85}"
ram_crit="${CBAR_SHOWCASE_RAM_CRIT:-95}"

read -r mem_total mem_available < <(
  awk '
    /^MemTotal:/ { total = $2 }
    /^MemAvailable:/ { available = $2 }
    END { print total, available }
  ' /proc/meminfo
)

used=$(( mem_total - mem_available ))
used_percent=$(( used * 100 / mem_total ))
used_gib="$(awk -v used="$used" 'BEGIN { printf "%.2f", used / 1048576 }')"
gauge_circumference="64.09"
gauge_dash="$(awk -v percent="$used_percent" -v circumference="$gauge_circumference" 'BEGIN { printf "%.2f", circumference * percent / 100 }')"
gauge_gap="$(awk -v dash="$gauge_dash" -v circumference="$gauge_circumference" 'BEGIN { printf "%.2f", circumference - dash }')"
color="#22d3ee"

if (( used_percent >= ram_crit )); then
  color="#f85149"
elif (( used_percent >= ram_warn )); then
  color="#f59e0b"
fi

svg_to_base64() {
  base64 | tr -d '\n'
}

gauge_image="$(
  cat <<SVG | svg_to_base64
<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 22 22">
  <circle cx="11" cy="11" r="10.2" fill="none" stroke="#5b6472" stroke-width="1.45"/>
  <circle cx="11" cy="11" r="10.2" fill="none" stroke="${color}" stroke-width="1.45"
          stroke-dasharray="${gauge_dash} ${gauge_gap}"
          stroke-linecap="round" transform="rotate(135 11 11)"/>
  <text x="11" y="12.7" text-anchor="middle" font-family="sans-serif"
        font-size="6.3" font-weight="600" fill="#e6edf3"
        textLength="13.2" lengthAdjust="spacingAndGlyphs">${used_gib}</text>
</svg>
SVG
)"

echo "| image=${gauge_image}"
echo "---"
echo "Memory usage"
echo "--Used: ${used_gib} GiB (${used_percent}%) | disabled=true"
echo "--Used: $(( used / 1024 )) MiB | disabled=true"
echo "--Available: $(( mem_available / 1024 )) MiB | disabled=true"
echo "--Warning threshold: ${ram_warn}% | disabled=true"
echo "--Critical threshold: ${ram_crit}% | disabled=true"
echo "Open memory details | shell=/bin/sh param1=-lc param2='free -h; printf \"\\n\"; read -r -p \"Press enter to close...\"' terminal=true"
