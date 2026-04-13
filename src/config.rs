use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub enabled_plugins: Option<BTreeSet<String>>,
}

impl AppConfig {
    pub fn is_enabled(&self, plugin_name: &str) -> bool {
        self.enabled_plugins
            .as_ref()
            .is_none_or(|plugins| plugins.contains(plugin_name))
    }
}

pub fn default_config_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config/cbar")
            .join(CONFIG_FILE_NAME);
    }

    PathBuf::from(CONFIG_FILE_NAME)
}

pub fn load_config(path: &Path) -> Result<AppConfig, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(AppConfig::default()),
        Err(err) => return Err(format!("failed to open config {}: {err}", path.display())),
    };

    let reader = BufReader::new(file);
    serde_json::from_reader(reader)
        .map_err(|err| format!("failed to parse config {}: {err}", path.display()))
}

pub fn save_config(path: PathBuf, config: AppConfig) -> Result<(), String> {
    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent)
        .map_err(|err| format!("failed to create config dir {}: {err}", parent.display()))?;

    let tmp_path = path.with_extension("json.tmp");
    let file = File::create(&tmp_path)
        .map_err(|err| format!("failed to create temp config {}: {err}", tmp_path.display()))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &config)
        .map_err(|err| format!("failed to serialize config {}: {err}", path.display()))?;
    fs::rename(&tmp_path, &path).map_err(|err| {
        format!(
            "failed to replace config {} with {}: {err}",
            path.display(),
            tmp_path.display()
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, load_config, save_config};
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_config_uses_defaults() {
        let path = unique_test_path("missing");
        let config = load_config(&path).expect("missing config should deserialize to defaults");
        assert!(config.enabled_plugins.is_none());
    }

    #[test]
    fn saves_and_loads_plugin_selection() {
        let path = unique_test_path("roundtrip");
        let mut enabled_plugins = BTreeSet::new();
        enabled_plugins.insert("demo.10s.sh".to_owned());
        enabled_plugins.insert("glados-monitor.30s.sh".to_owned());

        let config = AppConfig {
            enabled_plugins: Some(enabled_plugins.clone()),
        };

        save_config(path.clone(), config).expect("config should save");
        let loaded = load_config(&path).expect("config should load");

        assert_eq!(loaded.enabled_plugins, Some(enabled_plugins));
        cleanup_path(&path);
    }

    fn unique_test_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("cbar-config-tests-{label}-{nonce}"))
            .join("config.json")
    }

    fn cleanup_path(path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
