#!/usr/bin/env bash

set -euo pipefail

prefix="${CBAR_PREFIX:-$HOME/.local}"
bin_path="${prefix}/bin/cbar"
desktop_path="${prefix}/share/applications/io.github.alexprates.CBar.desktop"
icon_dir="${prefix}/share/icons/hicolor/scalable/apps"

rm -f \
  "${bin_path}" \
  "${desktop_path}" \
  "${icon_dir}/io.github.alexprates.CBar-symbolic.svg" \
  "${icon_dir}/io.github.alexprates.CBar.svg"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${prefix}/share/applications" || true
fi

printf 'Removed local cbar installation from %s.\n' "${prefix}"
