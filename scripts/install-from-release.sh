#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
repo_owner="${CBAR_REPO_OWNER:-alexandreprates}"
repo_name="${CBAR_REPO_NAME:-cbar}"
repo_slug="${repo_owner}/${repo_name}"
target="${CBAR_TARGET:-x86_64-unknown-linux-gnu}"
prefix="${CBAR_PREFIX:-$HOME/.local}"
bin_dir="${prefix}/bin"
app_dir="${prefix}/share/applications"
icon_dir="${prefix}/share/icons/hicolor/scalable/apps"
plugin_dir="${CBAR_PLUGIN_DIR:-$HOME/.config/cbar/plugins}"
binary_path="${bin_dir}/cbar"
desktop_template="${repo_root}/data/io.github.alexprates.CBar.desktop.in"
desktop_target="${app_dir}/io.github.alexprates.CBar.desktop"
release_api_url="https://api.github.com/repos/${repo_slug}/releases/latest"
archive_path="${1:-${CBAR_RELEASE_ARCHIVE:-}}"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Missing required command: %s\n' "$1" >&2
    exit 1
  fi
}

download_to() {
  local url="$1"
  local destination="$2"

  curl --fail --location --silent --show-error "$url" --output "$destination"
}

latest_release_json() {
  curl \
    --fail \
    --location \
    --silent \
    --show-error \
    -H "Accept: application/vnd.github+json" \
    "$release_api_url"
}

resolve_release_tag() {
  local release_json="$1"
  local tag

  tag="$(printf '%s' "$release_json" | tr -d '\n' | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"

  if [[ -z "$tag" ]]; then
    printf 'Failed to resolve the latest release tag from %s\n' "$release_api_url" >&2
    exit 1
  fi

  printf '%s\n' "$tag"
}

download_latest_archive() {
  local release_json
  local release_tag
  local asset_name
  local download_url
  local destination

  require_command curl
  require_command sed
  require_command tr

  release_json="$(latest_release_json)"
  release_tag="$(resolve_release_tag "$release_json")"
  asset_name="cbar-${release_tag}-${target}.tar.gz"
  download_url="https://github.com/${repo_slug}/releases/download/${release_tag}/${asset_name}"
  destination="${tmp_dir}/${asset_name}"

  download_to "$download_url" "$destination"

  printf '%s\n' "$destination"
}

resolve_or_download_archive_path() {
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

  if (( ${#matches[@]} > 1 )); then
    printf 'Found multiple local release archives in %s; downloading the latest published release instead.\n' "${repo_root}" >&2
  fi

  download_latest_archive
}

if [[ ! -f "${desktop_template}" ]]; then
  printf 'Desktop template not found: %s\n' "${desktop_template}" >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

archive_path="$(resolve_or_download_archive_path)"

if [[ ! -f "${archive_path}" ]]; then
  printf 'Release archive not found: %s\n' "${archive_path}" >&2
  exit 1
fi

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
