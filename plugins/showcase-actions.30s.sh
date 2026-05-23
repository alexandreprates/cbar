#!/usr/bin/env bash
# cbar: Demonstrates href, shell, terminal, and refresh actions.
# deps: date, printf, sh
# env: none

set -euo pipefail

echo "Actions"
echo "---"
echo "Open cbar plugins catalog | href=https://github.com/alexandreprates/cbar-plugins"
echo "Print timestamp | shell=/bin/sh param1=-lc param2='printf \"cbar action ran at %s\\n\" \"\$(date)\"' refresh=true"
echo "Open plugin directory | shell=xdg-open param1=${HOME}/.config/cbar/plugins"
echo "Run top in terminal | shell=/bin/sh param1=-lc param2='top' terminal=true"
echo "---"
echo "Action notes"
echo "--href opens URLs through xdg-open | disabled=true"
echo "--shell runs local commands as your user | disabled=true"
echo "--terminal=true requests an interactive terminal | disabled=true"
echo "--refresh=true reloads plugin output after action dispatch | disabled=true"
