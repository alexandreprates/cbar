#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
prefix="${CBAR_PREFIX:-$HOME/.local}"
bin_dir="${prefix}/bin"
app_dir="${prefix}/share/applications"
icon_dir="${prefix}/share/icons/hicolor/scalable/apps"
plugin_dir="${CBAR_PLUGIN_DIR:-$HOME/.config/cbar/plugins}"
binary_path="${bin_dir}/cbar"
desktop_template="${repo_root}/data/io.github.alexprates.CBar.desktop.in"
desktop_target="${app_dir}/io.github.alexprates.CBar.desktop"
archive_path="${1:-${CBAR_RELEASE_ARCHIVE:-}}"

resolve_archive_path() {
  if [[ -n "${archive_path}" ]]; then
    printf '%s\n' "${archive_path}"
    return 0
  fi

  shopt -s nullglob
  local matches=("${repo_root}"/cbar-*-x86_64-unknown-linux-gnu.tar.gz)
  shopt -u nullglob

  if (( ${#matches[@]} == 1 )); then
    printf '%s\n' "${matches[0]}"
    return 0
  fi

  printf 'Usage: %s <release-archive.tar.gz>\n' "${0}" >&2
  printf 'Or place exactly one cbar release archive in %s\n' "${repo_root}" >&2
  return 1
}

archive_path="$(resolve_archive_path)"

if [[ ! -f "${archive_path}" ]]; then
  printf 'Release archive not found: %s\n' "${archive_path}" >&2
  exit 1
fi

if [[ ! -f "${desktop_template}" ]]; then
  printf 'Desktop template not found: %s\n' "${desktop_template}" >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

tar -xzf "${archive_path}" -C "${tmp_dir}"

if [[ ! -f "${tmp_dir}/cbar" ]]; then
  printf 'Release archive does not contain the expected cbar binary.\n' >&2
  exit 1
fi

mkdir -p "${bin_dir}" "${app_dir}" "${icon_dir}" "${plugin_dir}"

install -m 0755 "${tmp_dir}/cbar" "${binary_path}"
install -m 0644 \
  "${repo_root}/data/icons/scalable/apps/io.github.alexprates.CBar-symbolic.svg" \
  "${icon_dir}/io.github.alexprates.CBar-symbolic.svg"
install -m 0644 \
  "${repo_root}/data/icons/scalable/apps/io.github.alexprates.CBar.svg" \
  "${icon_dir}/io.github.alexprates.CBar.svg"

sed "s|__CBAR_EXEC__|${binary_path}|g" "${desktop_template}" > "${desktop_target}"

if [[ ! -e "${plugin_dir}/demo.10s.sh" ]]; then
  install -m 0755 "${repo_root}/plugins/demo.10s.sh" "${plugin_dir}/demo.10s.sh"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${app_dir}" || true
fi

printf 'Installed cbar locally from release archive.\n'
printf 'Archive: %s\n' "${archive_path}"
printf 'Binary: %s\n' "${binary_path}"
printf 'Desktop entry: %s\n' "${desktop_target}"
printf 'Plugin dir: %s\n' "${plugin_dir}"
printf '\n'
printf 'If COSMIC does not pick up the applet right away, log out/in or restart the panel/session.\n'
