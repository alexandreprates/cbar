# cbar

`cbar` is a COSMIC applet for Pop!_OS inspired by xbar.

## Current scope

This first iteration implements:

- a libcosmic panel applet with a text button and popup menu
- plugin discovery from a directory
- periodic plugin execution based on xbar-like filename intervals
- parsing for a minimal xbar-compatible output subset:
  - panel title from the first line
  - `---` to split panel text from popup items
  - `--` nesting prefixes, rendered as indentation
  - inline parameters after `|`
  - `href=...`
  - `shell=...` / `bash=...`
  - `param1=...`, `param2=...`, ...
  - `refresh=true`

## Plugin directory

`cbar` resolves the plugin directory in this order:

1. `CBAR_PLUGIN_DIR`
2. `./plugins` when running from the repository
3. `~/.config/cbar/plugins`

## Example

An example plugin is provided in [plugins/demo.10s.sh](plugins/demo.10s.sh).

## Development

```bash
source ~/.cargo/env
cargo fmt
cargo test
cargo run
```

To test a custom directory:

```bash
CBAR_PLUGIN_DIR="$PWD/plugins" cargo run
```

## Local install for COSMIC

Install locally into `~/.local`:

```bash
cd /home/alexandreprates/Sources/projects/cbar
./scripts/install-local.sh
```

This installs:

- binary: `~/.local/bin/cbar`
- desktop entry: `~/.local/share/applications/io.github.alexprates.CBar.desktop`
- icons under `~/.local/share/icons/hicolor/scalable/apps/`
- example plugin in `~/.config/cbar/plugins/` if the file does not already exist

You can remove the local install with:

```bash
./scripts/uninstall-local.sh
```

Equivalent `just` targets are available:

```bash
just install-local
just uninstall-local
```

## Notes for testing in COSMIC

- The desktop entry is marked with `X-CosmicApplet=true`.
- COSMIC may require a panel/session restart or a new login before the applet appears in pickers.
- The applet defaults to `~/.config/cbar/plugins` when `CBAR_PLUGIN_DIR` is not set.
