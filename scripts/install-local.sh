#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
prefix="${CBAR_PREFIX:-$HOME/.local}"
bin_dir="${prefix}/bin"
app_dir="${prefix}/share/applications"
metainfo_dir="${prefix}/share/metainfo"
icon_dir="${prefix}/share/icons/hicolor/scalable/apps"
plugin_dir="${CBAR_PLUGIN_DIR:-$HOME/.config/cbar/plugins}"
binary_path="${bin_dir}/cbar"
desktop_template="${repo_root}/data/io.github.alexprates.CBar.desktop.in"
desktop_target="${app_dir}/io.github.alexprates.CBar.desktop"
metainfo_file="${repo_root}/data/io.github.alexprates.CBar.metainfo.xml"

mkdir -p "${bin_dir}" "${app_dir}" "${metainfo_dir}" "${icon_dir}" "${plugin_dir}"

if [[ -r "${HOME}/.cargo/env" ]]; then
  source "${HOME}/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
  printf 'Missing required command: cargo\n' >&2
  exit 1
fi

cargo install --path "${repo_root}" --root "${prefix}" --force

install -m 0644 \
  "${repo_root}/data/icons/scalable/apps/io.github.alexprates.CBar-symbolic.svg" \
  "${icon_dir}/io.github.alexprates.CBar-symbolic.svg"
install -m 0644 \
  "${repo_root}/data/icons/scalable/apps/io.github.alexprates.CBar.svg" \
  "${icon_dir}/io.github.alexprates.CBar.svg"

sed "s|__CBAR_EXEC__|${binary_path}|g" "${desktop_template}" > "${desktop_target}"
install -m 0644 "${metainfo_file}" "${metainfo_dir}/io.github.alexprates.CBar.metainfo.xml"

if [[ ! -e "${plugin_dir}/showcase-overview.10s.sh" ]]; then
  install -m 0755 "${repo_root}/plugins/showcase-overview.10s.sh" "${plugin_dir}/showcase-overview.10s.sh"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${app_dir}" || true
fi

printf 'Installed cbar locally.\n'
printf 'Binary: %s\n' "${binary_path}"
printf 'Desktop entry: %s\n' "${desktop_target}"
printf 'Metainfo: %s\n' "${metainfo_dir}/io.github.alexprates.CBar.metainfo.xml"
printf 'Plugin dir: %s\n' "${plugin_dir}"
printf '\n'
printf 'To test immediately in this shell:\n'
printf '  %s\n' "${binary_path}"
printf '\n'
printf 'If COSMIC does not pick up the applet right away, log out/in or restart the panel/session.\n'
