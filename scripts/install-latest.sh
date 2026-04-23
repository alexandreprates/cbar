#!/usr/bin/env bash

set -euo pipefail

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
desktop_target="${app_dir}/io.github.alexprates.CBar.desktop"
release_api_url="https://api.github.com/repos/${repo_slug}/releases/latest"
example_plugins=(
  "demo.10s.sh"
  "glados-monitor.30s.sh"
  "openai_codex.5m.sh"
)

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

install_optional_plugin() {
  local raw_base_url="$1"
  local plugin_name="$2"
  local destination="${plugin_dir}/${plugin_name}"
  local tmp_plugin="${tmp_dir}/${plugin_name}"

  if [[ -e "$destination" ]]; then
    printf 'Keeping existing plugin: %s\n' "$destination"
    return 0
  fi

  if download_to "${raw_base_url}/plugins/${plugin_name}" "$tmp_plugin"; then
    install -m 0755 "$tmp_plugin" "$destination"
    printf 'Installed example plugin: %s\n' "$destination"
  else
    rm -f "$tmp_plugin"
    printf 'Skipped unavailable example plugin for %s: %s\n' "$release_tag" "$plugin_name"
  fi
}

require_command curl
require_command install
require_command sed
require_command tar
require_command tr

if [[ "$(uname -s)" != "Linux" ]]; then
  printf 'This installer currently supports Linux only.\n' >&2
  exit 1
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

release_json="$(latest_release_json)"
release_tag="$(resolve_release_tag "$release_json")"
asset_name="cbar-${release_tag}-${target}.tar.gz"
archive_url="https://github.com/${repo_slug}/releases/download/${release_tag}/${asset_name}"
raw_base_url="https://raw.githubusercontent.com/${repo_slug}/${release_tag}"
archive_path="${tmp_dir}/${asset_name}"
desktop_template="${tmp_dir}/io.github.alexprates.CBar.desktop.in"
symbolic_icon="${tmp_dir}/io.github.alexprates.CBar-symbolic.svg"
app_icon="${tmp_dir}/io.github.alexprates.CBar.svg"

download_to "$archive_url" "$archive_path"
download_to "${raw_base_url}/data/io.github.alexprates.CBar.desktop.in" "$desktop_template"
download_to "${raw_base_url}/data/icons/scalable/apps/io.github.alexprates.CBar-symbolic.svg" "$symbolic_icon"
download_to "${raw_base_url}/data/icons/scalable/apps/io.github.alexprates.CBar.svg" "$app_icon"

tar -xzf "$archive_path" -C "$tmp_dir"

if [[ ! -f "${tmp_dir}/cbar" ]]; then
  printf 'Release archive does not contain the expected cbar binary.\n' >&2
  exit 1
fi

mkdir -p "$bin_dir" "$app_dir" "$icon_dir" "$plugin_dir"

install -m 0755 "${tmp_dir}/cbar" "$binary_path"
install -m 0644 "$symbolic_icon" "${icon_dir}/io.github.alexprates.CBar-symbolic.svg"
install -m 0644 "$app_icon" "${icon_dir}/io.github.alexprates.CBar.svg"

sed "s|__CBAR_EXEC__|${binary_path}|g" "$desktop_template" > "$desktop_target"

for plugin_name in "${example_plugins[@]}"; do
  install_optional_plugin "$raw_base_url" "$plugin_name"
done

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$app_dir" || true
fi

printf 'Installed cbar from the latest GitHub release.\n'
printf 'Repository: %s\n' "$repo_slug"
printf 'Release: %s\n' "$release_tag"
printf 'Binary: %s\n' "$binary_path"
printf 'Desktop entry: %s\n' "$desktop_target"
printf 'Plugin dir: %s\n' "$plugin_dir"
printf '\n'
printf 'Run with:\n'
printf '  %s\n' "$binary_path"
printf '\n'
printf 'If COSMIC does not pick up the applet right away, log out/in or restart the panel/session.\n'
