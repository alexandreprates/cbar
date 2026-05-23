# COSMIC community post draft

Target channel: https://chat.pop-os.org/pop-os/channels/cosmic-utils

```text
Hi folks! I built cbar, a scriptable COSMIC panel applet for running local plugins/scripts and rendering their output as live panel text plus popup actions.

Repo: https://github.com/alexandreprates/cbar
Plugin collection/catalog: https://github.com/alexandreprates/cbar-plugins

The current applet supports:
- local plugin discovery from ~/.config/cbar/plugins
- filename-based refresh intervals
- popup actions for links, shell commands, refreshes, and terminal tasks
- a curated plugin catalog that can be browsed and installed from the applet settings
- SHA-256 validation before installing catalog plugins

I started preparing it for COSMIC distribution:
- added AppStream metainfo with com.system76.CosmicApplet in <provides>
- added a Flatpak manifest following the patterns I saw in pop-os/cosmic-flatpak
- generated cargo-sources.json for offline Rust dependency builds

Before opening a PR against pop-os/cosmic-flatpak, I wanted to ask for guidance on the packaging model. cbar's core value is executing user-owned local scripts, and many useful plugins call host tools such as gh, docker, curl, ping, etc. That makes the Flatpak sandbox boundary an important design question.

Is pop-os/cosmic-flatpak the right target for this kind of scriptable applet, or would you recommend a different distribution approach for applets that intentionally execute local user scripts?

Any review expectations around permissions, metainfo, naming, or plugin execution would be very welcome.
```
