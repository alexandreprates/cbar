#!/usr/bin/env bash
# cbar: Demonstrates panel text, popup sections, nested rows, hidden rows, alternates, disabled rows, and refresh.
# deps: date
# env: none

set -euo pipefail

now="$(date '+%H:%M:%S')"

echo "cbar ${now}"
echo "Showcase cycle item"
echo "---"
echo "Panel output"
echo "--First non-empty line becomes the panel title | disabled=true"
echo "--Additional lines become cycle items | disabled=true"
echo "Popup output"
echo "--Lines after --- become popup rows | disabled=true"
echo "--Nested rows are rendered with indentation | disabled=true"
echo "Refresh this showcase | refresh=true"
echo "Primary row with alternate"
echo "[alt] Alternate row | alternate=true"
echo "Disabled row | disabled=true"
echo "Hidden row | dropdown=false"
echo "---"
echo "Open cbar repository | href=https://github.com/alexandreprates/cbar"
