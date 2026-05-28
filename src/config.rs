use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "config.json";
const ENV_FILE_NAME: &str = "env";

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

pub fn default_env_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config/cbar").join(ENV_FILE_NAME);
    }

    PathBuf::from(ENV_FILE_NAME)
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

pub fn load_env_file(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BTreeMap::new());
        }
        Err(err) => return Err(format!("failed to open env file {}: {err}", path.display())),
    };

    let mut env = BTreeMap::new();
    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.map_err(|err| {
            format!(
                "failed to read env file {} at line {}: {err}",
                path.display(),
                line_number
            )
        })?;

        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "invalid env assignment in {} at line {}",
                path.display(),
                line_number
            ));
        };

        let key = key.trim();
        if !is_env_key(key) {
            return Err(format!(
                "invalid env key '{}' in {} at line {}",
                key,
                path.display(),
                line_number
            ));
        }

        env.insert(key.to_owned(), parse_env_value(value.trim()));
    }

    Ok(env)
}

fn is_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn parse_env_value(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return value[1..value.len() - 1].to_owned();
        }
    }

    strip_unquoted_comment(value).trim_end().to_owned()
}

fn strip_unquoted_comment(value: &str) -> &str {
    let mut previous_was_whitespace = true;

    for (index, ch) in value.char_indices() {
        if ch == '#' && previous_was_whitespace {
            return &value[..index];
        }

        previous_was_whitespace = ch.is_whitespace();
    }

    value
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, load_config, load_env_file, save_config};
    use std::collections::BTreeSet;
    use std::fs::{self, File};
    use std::io::Write;
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
        enabled_plugins.insert("showcase-overview.10s.sh".to_owned());
        enabled_plugins.insert("showcase-status.5s.sh".to_owned());

        let config = AppConfig {
            enabled_plugins: Some(enabled_plugins.clone()),
        };

        save_config(path.clone(), config).expect("config should save");
        let loaded = load_config(&path).expect("config should load");

        assert_eq!(loaded.enabled_plugins, Some(enabled_plugins));
        cleanup_path(&path);
    }

    #[test]
    fn missing_env_file_uses_empty_env() {
        let path = unique_test_path("missing-env");
        let env = load_env_file(&path).expect("missing env file should deserialize to empty env");
        assert!(env.is_empty());
    }

    #[test]
    fn loads_env_assignments() {
        let path = unique_test_path("env").with_file_name("env");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("test env dir should be created");
        }

        let mut file = File::create(&path).expect("test env file should be created");
        writeln!(file, "# cbar env").expect("comment should write");
        writeln!(file, "CBAR_SSH_HOSTS=server,user@host").expect("env should write");
        writeln!(file, "CBAR_PING_HOST = \"1.1.1.1\"").expect("quoted env should write");
        writeln!(file, "CBAR_TIMER_SECONDS='900'").expect("single quoted env should write");
        writeln!(file, "CBAR_PUBLIC_IP_URL=https://example.test # comment")
            .expect("commented env should write");

        let env = load_env_file(&path).expect("env file should load");
        assert_eq!(
            env.get("CBAR_SSH_HOSTS").map(String::as_str),
            Some("server,user@host")
        );
        assert_eq!(
            env.get("CBAR_PING_HOST").map(String::as_str),
            Some("1.1.1.1")
        );
        assert_eq!(
            env.get("CBAR_TIMER_SECONDS").map(String::as_str),
            Some("900")
        );
        assert_eq!(
            env.get("CBAR_PUBLIC_IP_URL").map(String::as_str),
            Some("https://example.test")
        );
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
