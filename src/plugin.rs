use crate::parser::{
    EmbeddedImage, MenuEntry, ParsedPlugin, parse_plugin_output, parse_refresh_interval,
};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct PluginState {
    pub path: PathBuf,
    pub name: String,
    pub refresh_interval: Duration,
    pub next_refresh_at: Instant,
    pub last_output: ParsedPlugin,
    pub last_error: Option<String>,
}

impl PluginState {
    pub fn title(&self) -> &str {
        if self.last_output.title.is_empty() {
            &self.name
        } else {
            &self.last_output.title
        }
    }

    pub fn menu_entries(&self) -> &[MenuEntry] {
        &self.last_output.menu_entries
    }

    pub fn cycle_items(&self) -> &[String] {
        &self.last_output.cycle_items
    }

    pub fn title_image(&self) -> Option<&EmbeddedImage> {
        self.last_output.title_params.image.as_ref()
    }
}

pub async fn load_plugins(dir: PathBuf) -> Vec<PluginState> {
    let mut plugins = discover_plugins(&dir);
    for plugin in &mut plugins {
        refresh_plugin(plugin).await;
    }
    plugins
}

pub async fn refresh_plugin_state(mut plugin: PluginState) -> PluginState {
    refresh_plugin(&mut plugin).await;
    plugin
}

pub async fn trigger_entry(plugin: &PluginState, entry: &MenuEntry) -> Result<bool, String> {
    if let Some(href) = &entry.params.href {
        Command::new("xdg-open")
            .arg(href)
            .spawn()
            .map_err(|err| format!("failed to open href: {err}"))?;
        return Ok(entry.params.refresh);
    }

    if let Some(shell) = &entry.params.shell {
        spawn_action_command(
            plugin,
            shell.as_str(),
            &entry.params.params,
            entry.params.terminal,
        )?;
        return Ok(entry.params.refresh);
    }

    if !entry.params.params.is_empty() {
        spawn_action_command(
            plugin,
            plugin.path.to_string_lossy().as_ref(),
            &entry.params.params,
            entry.params.terminal,
        )?;
        return Ok(entry.params.refresh);
    }

    Ok(false)
}

fn spawn_action_command(
    plugin: &PluginState,
    executable: &str,
    args: &[String],
    terminal: bool,
) -> Result<(), String> {
    let plugin_dir = plugin.path.parent().unwrap_or(Path::new("."));

    if terminal {
        let mut command = Command::new("x-terminal-emulator");
        command.arg("-e");
        command.arg(executable);
        command.args(args);
        command.current_dir(plugin_dir);
        command.envs(std::env::vars());
        command
            .spawn()
            .map_err(|err| format!("failed to spawn terminal action: {err}"))?;
        return Ok(());
    }

    let mut command = Command::new(executable);
    command.args(args);
    command.current_dir(plugin_dir);
    command.envs(std::env::vars());
    command
        .spawn()
        .map_err(|err| format!("failed to spawn action: {err}"))?;

    Ok(())
}

fn discover_plugins(dir: &Path) -> Vec<PluginState> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut plugins = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
                return None;
            }

            let name = path.file_name()?.to_string_lossy().into_owned();
            let refresh_interval = parse_refresh_interval(&name);

            Some(PluginState {
                path,
                name,
                refresh_interval,
                next_refresh_at: Instant::now(),
                last_output: ParsedPlugin::default(),
                last_error: None,
            })
        })
        .collect::<Vec<_>>();

    plugins.sort_by(|a, b| a.name.cmp(&b.name));
    plugins
}

async fn refresh_plugin(plugin: &mut PluginState) {
    match run_plugin(&plugin.path).await {
        Ok(output) => {
            plugin.last_output = parse_plugin_output(&output);
            plugin.last_error = None;
        }
        Err(err) => {
            plugin.last_output = ParsedPlugin {
                title: format!("{} !", plugin.name),
                title_params: Default::default(),
                cycle_items: vec![format!("{} !", plugin.name)],
                menu_entries: Vec::new(),
            };
            plugin.last_error = Some(err);
        }
    }

    plugin.next_refresh_at = Instant::now() + plugin.refresh_interval;
}

async fn run_plugin(path: &Path) -> Result<String, String> {
    let output = Command::new(path)
        .output()
        .await
        .map_err(|err| format!("failed to execute plugin: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let message = if stderr.is_empty() {
            format!("plugin exited with status {}", output.status)
        } else {
            stderr
        };
        return Err(message);
    }

    String::from_utf8(output.stdout).map_err(|err| format!("invalid utf-8 output: {err}"))
}
