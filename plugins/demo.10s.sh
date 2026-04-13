#!/usr/bin/env bash

set -euo pipefail

now="$(date +%H:%M:%S)"

echo "cbar ${now}"
echo "---"
echo "Open COSMIC repo | href=https://github.com/pop-os/cosmic-applets"
echo "Print hello | shell=/bin/sh | param1=-lc | param2='printf hello-from-cbar'"
echo "--Nested item"
echo "-----"
echo "Refresh plugins | refresh=true"
