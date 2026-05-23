<p align="center">
  <img src="data/icons/scalable/apps/io.github.alexprates.CBar.svg" width="96" alt="cbar icon">
</p>

# cbar

[![License: GPL-3.0-only](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.94%2B-orange.svg)](https://www.rust-lang.org/)
[![COSMIC](https://img.shields.io/badge/COSMIC-applet-7c3aed.svg)](https://system76.com/cosmic)

`cbar` is a [COSMIC](https://system76.com/cosmic) panel applet for Pop!_OS that runs local scripts and turns their output into live panel text and interactive popup actions.

It is designed for small, scriptable workflows that belong in the desktop panel:

- show status from local commands or web APIs
- open links and tools from a lightweight popup
- run shell actions without building a full desktop app
- refresh each plugin on its own interval
- keep plugin behavior transparent and easy to inspect

`cbar` is built around COSMIC and `libcosmic` applet behavior.

## Status

`cbar` is currently ready for early adopters and community feedback.

It supports:

- a working `libcosmic` panel applet
- plugin discovery from a local directory
- periodic plugin execution based on filename intervals
- per-plugin parallel refresh with in-flight protection
- popup menu actions for links, shell commands, refreshes, and terminal tasks
- local installation for testing in a COSMIC session

The implementation is intentionally focused on practical COSMIC integration.

## Features

- COSMIC panel applet built with `libcosmic`
- text-based panel label composed from plugin output
- popup menu generated from plugin stdout
- per-plugin refresh scheduling
- per-plugin parallel refresh with per-plugin in-flight protection
- local plugin execution with support for shell actions, URLs, and terminal actions
- plugin catalog browsing and installation from the applet settings
- local install script for user-level testing

## Plugin Output Format

Plugins write plain text to stdout. The first visible line becomes panel text, and lines after `---` become popup entries.

Supported item syntax:

- first line as panel title
- `---` to separate panel output from popup items
- `--` for submenu depth, rendered as indentation in the popup
- `href=...`
- `shell=...` / `bash=...`
- `param1=...`, `param2=...`, ...
- `refresh=true`
- `terminal=true`
- `dropdown=false`
- `alternate=true`
- `disabled=true`
- `trim=true|false`

Notes:

- `alternate=true` is adapted for COSMIC by rendering an explicit `[alt]` entry in the popup. Modifier-key alternate entries do not map directly to this applet UI.
- nested menu items are currently rendered as indented entries, not true nested popup submenus.

## Inspiration

The plugin output format is inspired by [xbar](https://github.com/matryer/xbar), while the applet itself is designed specifically for COSMIC.

## Plugin Directory

`cbar` resolves the plugin directory in this order:

1. `CBAR_PLUGIN_DIR`
2. `./plugins` when running from the repository
3. `~/.config/cbar/plugins`

An example plugin is included at [plugins/demo.10s.sh](plugins/demo.10s.sh).

## Plugin Catalog

The applet settings include a plugin catalog view backed by the public [cbar-plugins](https://github.com/alexandreprates/cbar-plugins) repository.

Catalog installation downloads the selected plugin script into the active plugin directory, validates the published SHA-256 checksum, marks the file executable, and reloads the plugin list. Catalog plugins are local scripts and run as the current user.

By default, `cbar` loads:

```text
https://raw.githubusercontent.com/alexandreprates/cbar-plugins/main/registry/plugins.json
```

To test another registry:

```bash
CBAR_PLUGIN_REGISTRY_URL="https://example.com/plugins.json" cargo run
```

## Example Plugin

```bash
#!/usr/bin/env bash

echo "Workday"
echo "---"
echo "Open dashboard | href=https://example.com"
echo "Run sync | bash=/bin/bash param1=-lc param2='echo syncing...' refresh=true"
echo "Open terminal task | bash=/bin/bash param1=-lc param2='htop' terminal=true"
echo "Hidden helper | dropdown=false"
echo "Disabled item | disabled=true"
```

## Build Requirements

System dependencies commonly needed for local development on Pop!_OS:

```bash
sudo apt update
sudo apt install -y cmake just libexpat1-dev libfontconfig-dev libfreetype-dev libxkbcommon-dev pkgconf curl
```

Rust is expected to be installed through `rustup`:

```bash
curl https://sh.rustup.rs -sSf | sh
source ~/.cargo/env
```

## Running from Source

```bash
source ~/.cargo/env
cargo run
```

To force a specific plugin directory:

```bash
CBAR_PLUGIN_DIR="$PWD/plugins" cargo run
```

## Local Installation for COSMIC

Install locally into `~/.local`:

```bash
./scripts/install-local.sh
```

This installs:

- `~/.local/bin/cbar`
- `~/.local/share/applications/io.github.alexprates.CBar.desktop`
- icons under `~/.local/share/icons/hicolor/scalable/apps/`
- the example plugin in `~/.config/cbar/plugins/` if it does not already exist

You can remove the local installation with:

```bash
./scripts/uninstall-local.sh
```

Equivalent `just` targets are available:

```bash
just install-local
just uninstall-local
```

## Installing from a GitHub Release

If you want to install the prebuilt release binary without compiling locally, clone the repository and let the installer resolve the latest published release automatically:

```bash
git clone git@github.com:alexandreprates/cbar.git
cd cbar
./scripts/install-from-release.sh
```

If you already downloaded a release archive, you can still pass it explicitly:

```bash
git clone git@github.com:alexandreprates/cbar.git
cd cbar

version="v1.2.1"
archive="cbar-${version}-x86_64-unknown-linux-gnu.tar.gz"

curl -LO "https://github.com/alexandreprates/cbar/releases/download/${version}/${archive}"
./scripts/install-from-release.sh "./${archive}"
```

The script also accepts the archive path through `CBAR_RELEASE_ARCHIVE`:

```bash
CBAR_RELEASE_ARCHIVE=./cbar-v1.2.1-x86_64-unknown-linux-gnu.tar.gz ./scripts/install-from-release.sh
```

Without an explicit archive path, `scripts/install-from-release.sh` first looks for exactly one local `cbar-*-x86_64-unknown-linux-gnu.tar.gz` file in the repository root. If none is available, it queries the latest GitHub release, resolves the current tag, and downloads the matching versioned archive automatically.

The installer reuses the desktop entry and icons from the repository and installs:

- `~/.local/bin/cbar`
- `~/.local/share/applications/io.github.alexprates.CBar.desktop`
- icons under `~/.local/share/icons/hicolor/scalable/apps/`
- the example plugin in `~/.config/cbar/plugins/` if it does not already exist

If you prefer a direct installation from GitHub without cloning the repository first, run:

```bash
curl -fsSL https://raw.githubusercontent.com/alexandreprates/cbar/main/scripts/install-latest.sh | bash
```

The remote installer resolves the latest published release automatically, installs the binary and desktop assets under `~/.local`, creates `~/.config/cbar/plugins`, and copies the example plugins that exist for that tagged release.

If the archive is placed in the repository root, the repository installer can autodetect it as well:

```bash
./scripts/install-from-release.sh
```

## Testing in COSMIC

The local desktop entry is installed with:

- `X-CosmicApplet=true`
- `X-CosmicHoverPopup=Auto`

Depending on the session state, COSMIC may require:

- logging out and back in
- restarting the panel or session

before the new applet entry becomes visible in applet pickers or behaves consistently.

## Development

Common commands:

```bash
source ~/.cargo/env
cargo fmt
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Or via `just`:

```bash
just test
```

## Project Structure

```text
src/app.rs        COSMIC applet state, messages, popup UI, refresh orchestration
src/plugin.rs     plugin discovery, execution, action dispatch
src/parser.rs     plugin output parsing
data/             desktop entry template and icons
packaging/        Flatpak manifest for local packaging and COSMIC submission prep
scripts/          local install and uninstall scripts
plugins/          example plugins for development
```

## Limitations

- nested popup items are visual indentation only
- plugin discovery is static after startup
- no persistent plugin configuration UI yet
- some visual plugin item parameters are not mapped to COSMIC UI yet

## Roadmap

- add dynamic plugin discovery
- support more plugin item parameters where they fit COSMIC
- improve popup hierarchy and action presentation
- package the applet for easier installation

## Related Projects

- [cosmic-applets](https://github.com/pop-os/cosmic-applets)
- [libcosmic](https://github.com/pop-os/libcosmic)

## License

GPL-3.0-only
