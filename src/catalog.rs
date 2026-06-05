use crate::config::default_env_path;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, ErrorKind, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/alexandreprates/cbar-plugins/main/registry/plugins.json";
const MAX_PLUGIN_DOWNLOAD_BYTES: u64 = 1024 * 1024;
const INSTALLATIONS_FILE_NAME: &str = "catalog-installations.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginRegistry {
    pub version: u32,
    pub repository: String,
    pub raw_base_url: String,
    pub plugins: Vec<CatalogPlugin>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogPlugin {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    #[serde(default = "default_plugin_version")]
    pub plugin_version: String,
    pub path: String,
    pub download_url: String,
    pub install_name: String,
    pub interval: String,
    pub language: String,
    #[serde(default)]
    pub languages: Vec<String>,
    pub dependencies: Vec<String>,
    pub env: Vec<String>,
    pub sha256: String,
    pub size_bytes: u64,
    pub license: String,
    pub publisher: Option<String>,
    pub publisher_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CatalogInstallation {
    pub plugin_id: String,
    pub install_name: String,
    pub installed_version: String,
    pub installed_sha256: String,
    pub registry_url: String,
    pub installed_at: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct CatalogInstallations {
    pub plugins: BTreeMap<String, CatalogInstallation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogInstallResult {
    pub plugin_id: String,
    pub install_name: String,
    pub installation: CatalogInstallation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogRemoveResult {
    pub plugin_id: String,
    pub install_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogPluginInstallState {
    Available,
    Installed,
    UpdateAvailable,
    Modified,
    Unmanaged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogPluginInstallStatus {
    pub state: CatalogPluginInstallState,
    pub installed_version: Option<String>,
}

impl CatalogPlugin {
    pub fn installed_path(&self, plugin_dir: &Path) -> Result<PathBuf, String> {
        if self.install_name.contains('/') || self.install_name.contains('\\') {
            return Err(format!(
                "invalid plugin install name: {}",
                self.install_name
            ));
        }

        Ok(plugin_dir.join(&self.install_name))
    }

    fn installation_record(&self) -> CatalogInstallation {
        CatalogInstallation {
            plugin_id: self.id.clone(),
            install_name: self.install_name.clone(),
            installed_version: self.plugin_version.clone(),
            installed_sha256: self.sha256.clone(),
            registry_url: catalog_registry_url(),
            installed_at: current_unix_timestamp(),
        }
    }
}

pub async fn fetch_catalog() -> Result<Vec<CatalogPlugin>, String> {
    let registry_url = catalog_registry_url();

    let response = reqwest::Client::new()
        .get(&registry_url)
        .send()
        .await
        .map_err(|err| format!("failed to fetch plugin catalog: {err}"))?
        .error_for_status()
        .map_err(|err| format!("plugin catalog request failed: {err}"))?;

    let registry = response
        .json::<PluginRegistry>()
        .await
        .map_err(|err| format!("failed to parse plugin catalog: {err}"))?;

    if registry.version != 1 {
        return Err(format!(
            "unsupported plugin catalog version {}",
            registry.version
        ));
    }

    for plugin in &registry.plugins {
        validate_plugin_version(plugin)?;
    }

    Ok(registry.plugins)
}

pub async fn install_catalog_plugin(
    plugin_dir: PathBuf,
    plugin: CatalogPlugin,
) -> Result<CatalogInstallResult, String> {
    let destination = plugin.installed_path(&plugin_dir)?;
    validate_declared_size(&plugin)?;

    if destination.exists() {
        return Err(format!("plugin already exists: {}", destination.display()));
    }

    let bytes = download_catalog_plugin(&plugin).await?;
    validate_plugin_checksum(&plugin, &bytes)?;
    ensure_catalog_plugin_env_entries(&plugin, &default_env_path())?;

    fs::create_dir_all(&plugin_dir).map_err(|err| {
        format!(
            "failed to create plugin directory {}: {err}",
            plugin_dir.display()
        )
    })?;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|err| {
            if err.kind() == ErrorKind::AlreadyExists {
                format!("plugin already exists: {}", destination.display())
            } else {
                format!("failed to create plugin {}: {err}", destination.display())
            }
        })?;
    file.write_all(&bytes)
        .map_err(|err| format!("failed to write plugin {}: {err}", destination.display()))?;

    mark_plugin_executable(&destination)?;

    Ok(CatalogInstallResult {
        plugin_id: plugin.id.clone(),
        install_name: plugin.install_name.clone(),
        installation: plugin.installation_record(),
    })
}

pub async fn update_catalog_plugin(
    plugin_dir: PathBuf,
    plugin: CatalogPlugin,
    installation: CatalogInstallation,
) -> Result<CatalogInstallResult, String> {
    let destination = plugin.installed_path(&plugin_dir)?;
    validate_declared_size(&plugin)?;
    if installation.plugin_id != plugin.id || installation.install_name != plugin.install_name {
        return Err(format!(
            "catalog installation metadata does not match {}",
            plugin.install_name
        ));
    }

    let metadata = fs::metadata(&destination).map_err(|err| {
        if err.kind() == ErrorKind::NotFound {
            format!("plugin is not installed: {}", destination.display())
        } else {
            format!(
                "failed to read plugin metadata {}: {err}",
                destination.display()
            )
        }
    })?;

    if !metadata.is_file() {
        return Err(format!(
            "installed plugin is not a file: {}",
            destination.display()
        ));
    }

    let current_bytes = fs::read(&destination)
        .map_err(|err| format!("failed to read plugin {}: {err}", destination.display()))?;
    let current_sha256 = sha256_hex(&current_bytes);
    if current_sha256 != installation.installed_sha256 {
        return Err(format!(
            "plugin has local changes and was not overwritten: {}",
            destination.display()
        ));
    }

    let bytes = download_catalog_plugin(&plugin).await?;
    validate_plugin_checksum(&plugin, &bytes)?;
    ensure_catalog_plugin_env_entries(&plugin, &default_env_path())?;

    let tmp_destination = destination.with_file_name(format!(".{}.tmp", plugin.install_name));
    fs::write(&tmp_destination, &bytes).map_err(|err| {
        format!(
            "failed to write temp plugin {}: {err}",
            tmp_destination.display()
        )
    })?;
    mark_plugin_executable(&tmp_destination)?;
    fs::rename(&tmp_destination, &destination).map_err(|err| {
        let _ = fs::remove_file(&tmp_destination);
        format!(
            "failed to replace plugin {} with {}: {err}",
            destination.display(),
            tmp_destination.display()
        )
    })?;

    Ok(CatalogInstallResult {
        plugin_id: plugin.id.clone(),
        install_name: plugin.install_name.clone(),
        installation: plugin.installation_record(),
    })
}

pub async fn remove_catalog_plugin(
    plugin_dir: PathBuf,
    plugin: CatalogPlugin,
) -> Result<CatalogRemoveResult, String> {
    let destination = plugin.installed_path(&plugin_dir)?;
    let metadata = fs::metadata(&destination).map_err(|err| {
        if err.kind() == ErrorKind::NotFound {
            format!("plugin is not installed: {}", destination.display())
        } else {
            format!(
                "failed to read plugin metadata {}: {err}",
                destination.display()
            )
        }
    })?;

    if !metadata.is_file() {
        return Err(format!(
            "installed plugin is not a file: {}",
            destination.display()
        ));
    }

    fs::remove_file(&destination)
        .map_err(|err| format!("failed to remove plugin {}: {err}", destination.display()))?;

    Ok(CatalogRemoveResult {
        plugin_id: plugin.id,
        install_name: plugin.install_name,
    })
}

async fn download_catalog_plugin(plugin: &CatalogPlugin) -> Result<Vec<u8>, String> {
    let response = reqwest::Client::new()
        .get(&plugin.download_url)
        .send()
        .await
        .map_err(|err| format!("failed to download plugin: {err}"))?
        .error_for_status()
        .map_err(|err| format!("plugin download failed: {err}"))?;

    read_plugin_response(response, plugin).await
}

async fn read_plugin_response(
    mut response: reqwest::Response,
    plugin: &CatalogPlugin,
) -> Result<Vec<u8>, String> {
    if let Some(content_length) = response.content_length() {
        validate_download_size(plugin, content_length)?;
    }

    let mut bytes = Vec::with_capacity(plugin.size_bytes as usize);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| format!("failed to read plugin download: {err}"))?
    {
        bytes.extend_from_slice(&chunk);
        validate_download_size(plugin, bytes.len() as u64)?;
    }

    if bytes.len() as u64 != plugin.size_bytes {
        return Err(format!(
            "plugin size mismatch for {}: expected {} bytes, got {} bytes",
            plugin.install_name,
            plugin.size_bytes,
            bytes.len()
        ));
    }

    Ok(bytes)
}

fn validate_declared_size(plugin: &CatalogPlugin) -> Result<(), String> {
    if plugin.size_bytes == 0 {
        return Err(format!(
            "invalid plugin size for {}: size must be greater than zero",
            plugin.install_name
        ));
    }

    validate_download_size(plugin, plugin.size_bytes)
}

fn validate_download_size(plugin: &CatalogPlugin, size_bytes: u64) -> Result<(), String> {
    if size_bytes > MAX_PLUGIN_DOWNLOAD_BYTES {
        return Err(format!(
            "plugin {} is too large: {} bytes exceeds the {} byte limit",
            plugin.install_name, size_bytes, MAX_PLUGIN_DOWNLOAD_BYTES
        ));
    }

    if size_bytes > plugin.size_bytes {
        return Err(format!(
            "plugin {} is larger than the catalog metadata: {} bytes exceeds {} bytes",
            plugin.install_name, size_bytes, plugin.size_bytes
        ));
    }

    Ok(())
}

fn validate_plugin_checksum(plugin: &CatalogPlugin, bytes: &[u8]) -> Result<(), String> {
    let actual_sha256 = sha256_hex(bytes);
    if actual_sha256 != plugin.sha256 {
        return Err(format!(
            "plugin checksum mismatch for {}",
            plugin.install_name
        ));
    }

    Ok(())
}

fn validate_plugin_version(plugin: &CatalogPlugin) -> Result<(), String> {
    Version::parse(&plugin.plugin_version)
        .map_err(|err| format!("invalid plugin version for {}: {err}", plugin.install_name))?;

    Ok(())
}

fn mark_plugin_executable(path: &Path) -> Result<(), String> {
    let mut permissions = fs::metadata(path)
        .map_err(|err| format!("failed to read plugin metadata {}: {err}", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|err| format!("failed to mark plugin executable {}: {err}", path.display()))
}

fn ensure_catalog_plugin_env_entries(plugin: &CatalogPlugin, path: &Path) -> Result<(), String> {
    if plugin.env.is_empty() {
        return Ok(());
    }

    let existing_keys = read_existing_env_keys(path)?;
    let missing_keys = plugin
        .env
        .iter()
        .filter(|key| is_catalog_env_key(key))
        .filter(|key| !existing_keys.contains(*key))
        .collect::<Vec<_>>();

    if missing_keys.is_empty() {
        return Ok(());
    }

    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent).map_err(|err| {
        format!(
            "failed to create env file directory {}: {err}",
            parent.display()
        )
    })?;

    let needs_leading_newline = fs::metadata(path).is_ok_and(|metadata| metadata.len() > 0);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("failed to open env file {}: {err}", path.display()))?;

    if needs_leading_newline {
        writeln!(file)
            .map_err(|err| format!("failed to write env file {}: {err}", path.display()))?;
    }

    writeln!(
        file,
        "# Environment variables for {} catalog plugin.",
        plugin.name
    )
    .map_err(|err| format!("failed to write env file {}: {err}", path.display()))?;
    for key in missing_keys {
        writeln!(file, "# {key}=")
            .map_err(|err| format!("failed to write env file {}: {err}", path.display()))?;
    }

    Ok(())
}

fn read_existing_env_keys(path: &Path) -> Result<std::collections::BTreeSet<String>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Ok(std::collections::BTreeSet::new());
        }
        Err(err) => return Err(format!("failed to open env file {}: {err}", path.display())),
    };

    let mut keys = std::collections::BTreeSet::new();
    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.map_err(|err| {
            format!(
                "failed to read env file {} at line {}: {err}",
                path.display(),
                line_number
            )
        })?;
        if let Some(key) = env_key_from_line(&line) {
            keys.insert(key.to_owned());
        }
    }

    Ok(keys)
}

fn env_key_from_line(line: &str) -> Option<&str> {
    let line = line.trim();
    let line = line.strip_prefix('#').map(str::trim_start).unwrap_or(line);
    let (key, _) = line.split_once('=')?;
    let key = key.trim();
    is_catalog_env_key(key).then_some(key)
}

fn is_catalog_env_key(key: &str) -> bool {
    key.strip_prefix("CBAR_").is_some_and(|suffix| {
        !suffix.is_empty()
            && suffix
                .chars()
                .all(|ch| ch == '_' || ch.is_ascii_uppercase() || ch.is_ascii_digit())
    })
}

pub fn default_catalog_installations_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config/cbar")
            .join(INSTALLATIONS_FILE_NAME);
    }

    PathBuf::from(INSTALLATIONS_FILE_NAME)
}

pub fn catalog_registry_url() -> String {
    std::env::var("CBAR_PLUGIN_REGISTRY_URL").unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_owned())
}

pub fn load_catalog_installations(path: &Path) -> Result<CatalogInstallations, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Ok(CatalogInstallations::default());
        }
        Err(err) => {
            return Err(format!(
                "failed to open catalog installations {}: {err}",
                path.display()
            ));
        }
    };

    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|err| {
        format!(
            "failed to parse catalog installations {}: {err}",
            path.display()
        )
    })
}

pub fn save_catalog_installations(
    path: PathBuf,
    installations: CatalogInstallations,
) -> Result<(), String> {
    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent).map_err(|err| {
        format!(
            "failed to create catalog installations dir {}: {err}",
            parent.display()
        )
    })?;

    let tmp_path = path.with_extension("json.tmp");
    let file = File::create(&tmp_path).map_err(|err| {
        format!(
            "failed to create temp catalog installations {}: {err}",
            tmp_path.display()
        )
    })?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &installations).map_err(|err| {
        format!(
            "failed to serialize catalog installations {}: {err}",
            path.display()
        )
    })?;
    fs::rename(&tmp_path, &path).map_err(|err| {
        format!(
            "failed to replace catalog installations {} with {}: {err}",
            path.display(),
            tmp_path.display()
        )
    })?;

    Ok(())
}

pub fn reconcile_catalog_installations(
    installations: &mut CatalogInstallations,
    plugins: &[CatalogPlugin],
    plugin_dir: &Path,
) -> bool {
    let mut changed = false;

    for plugin in plugins {
        let Ok(destination) = plugin.installed_path(plugin_dir) else {
            continue;
        };

        if installations
            .plugins
            .get(&plugin.id)
            .is_some_and(|installation| {
                installation.install_name == plugin.install_name && !destination.exists()
            })
        {
            installations.plugins.remove(&plugin.id);
            changed = true;
            continue;
        }

        if installations.plugins.contains_key(&plugin.id) || !destination.exists() {
            continue;
        }

        let Ok(bytes) = fs::read(&destination) else {
            continue;
        };

        if sha256_hex(&bytes) == plugin.sha256 {
            installations
                .plugins
                .insert(plugin.id.clone(), plugin.installation_record());
            changed = true;
        }
    }

    changed
}

pub fn catalog_plugin_install_status(
    plugin: &CatalogPlugin,
    plugin_dir: &Path,
    installations: &CatalogInstallations,
) -> CatalogPluginInstallStatus {
    let Ok(destination) = plugin.installed_path(plugin_dir) else {
        return CatalogPluginInstallStatus {
            state: CatalogPluginInstallState::Available,
            installed_version: None,
        };
    };

    if !destination.exists() {
        return CatalogPluginInstallStatus {
            state: CatalogPluginInstallState::Available,
            installed_version: None,
        };
    }

    let current_sha256 = fs::read(&destination).ok().map(|bytes| sha256_hex(&bytes));

    if let Some(installation) = installations.plugins.get(&plugin.id)
        && installation.install_name == plugin.install_name
    {
        let installed_version = Some(installation.installed_version.clone());

        if current_sha256.as_deref() != Some(installation.installed_sha256.as_str()) {
            return CatalogPluginInstallStatus {
                state: CatalogPluginInstallState::Modified,
                installed_version,
            };
        }

        let state =
            if is_newer_plugin_version(&plugin.plugin_version, &installation.installed_version) {
                CatalogPluginInstallState::UpdateAvailable
            } else {
                CatalogPluginInstallState::Installed
            };

        return CatalogPluginInstallStatus {
            state,
            installed_version,
        };
    }

    if current_sha256.as_deref() == Some(plugin.sha256.as_str()) {
        return CatalogPluginInstallStatus {
            state: CatalogPluginInstallState::Installed,
            installed_version: Some(plugin.plugin_version.clone()),
        };
    }

    CatalogPluginInstallStatus {
        state: CatalogPluginInstallState::Unmanaged,
        installed_version: None,
    }
}

fn is_newer_plugin_version(remote_version: &str, installed_version: &str) -> bool {
    let Ok(remote_version) = Version::parse(remote_version) else {
        return false;
    };
    let Ok(installed_version) = Version::parse(installed_version) else {
        return false;
    };

    remote_version.cmp_precedence(&installed_version) == Ordering::Greater
}

fn default_plugin_version() -> String {
    "0.0.0".to_owned()
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);

    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }

    hex
}

#[cfg(test)]
mod tests {
    use super::{
        CatalogInstallation, CatalogInstallations, CatalogPlugin, CatalogPluginInstallState,
        MAX_PLUGIN_DOWNLOAD_BYTES, catalog_plugin_install_status,
        ensure_catalog_plugin_env_entries, reconcile_catalog_installations, remove_catalog_plugin,
        sha256_hex, validate_declared_size, validate_plugin_version,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn rejects_install_names_with_path_separators() {
        let plugin = CatalogPlugin {
            id: "test.bad".to_owned(),
            name: "Bad".to_owned(),
            category: "test".to_owned(),
            description: "Bad path".to_owned(),
            plugin_version: "1.0.0".to_owned(),
            path: "plugins/bad.sh".to_owned(),
            download_url: "https://example.com/bad.sh".to_owned(),
            install_name: "../bad.sh".to_owned(),
            interval: "1m".to_owned(),
            language: "bash".to_owned(),
            languages: Vec::new(),
            dependencies: Vec::new(),
            env: Vec::new(),
            sha256: String::new(),
            size_bytes: 0,
            license: "GPL-3.0-only".to_owned(),
            publisher: None,
            publisher_url: None,
        };

        assert!(plugin.installed_path(Path::new("/tmp/cbar")).is_err());
    }

    #[test]
    fn computes_sha256_hex() {
        assert_eq!(
            sha256_hex(b"cbar"),
            "a51b7d32cf572b9468acfde8d65a984bf4a09d4a7810d1fbffcba8025dbb94fa"
        );
    }

    #[test]
    fn rejects_empty_or_oversized_catalog_plugins() {
        let mut plugin = CatalogPlugin {
            id: "test.large".to_owned(),
            name: "Large".to_owned(),
            category: "test".to_owned(),
            description: "Large plugin".to_owned(),
            plugin_version: "1.0.0".to_owned(),
            path: "plugins/large.sh".to_owned(),
            download_url: "https://example.com/large.sh".to_owned(),
            install_name: "large.sh".to_owned(),
            interval: "1m".to_owned(),
            language: "bash".to_owned(),
            languages: Vec::new(),
            dependencies: Vec::new(),
            env: Vec::new(),
            sha256: String::new(),
            size_bytes: 0,
            license: "GPL-3.0-only".to_owned(),
            publisher: None,
            publisher_url: None,
        };

        assert!(validate_declared_size(&plugin).is_err());

        plugin.size_bytes = MAX_PLUGIN_DOWNLOAD_BYTES + 1;
        assert!(validate_declared_size(&plugin).is_err());
    }

    #[test]
    fn removes_installed_catalog_plugin() {
        let plugin_dir = unique_test_dir("remove");
        fs::create_dir_all(&plugin_dir).expect("plugin dir should be created");
        let plugin_path = plugin_dir.join("remove-me.sh");
        fs::write(&plugin_path, b"#!/usr/bin/env bash\n").expect("plugin should be written");

        let plugin = CatalogPlugin {
            id: "test.remove".to_owned(),
            name: "Remove".to_owned(),
            category: "test".to_owned(),
            description: "Remove plugin".to_owned(),
            plugin_version: "1.0.0".to_owned(),
            path: "plugins/remove-me.sh".to_owned(),
            download_url: "https://example.com/remove-me.sh".to_owned(),
            install_name: "remove-me.sh".to_owned(),
            interval: "1m".to_owned(),
            language: "bash".to_owned(),
            languages: Vec::new(),
            dependencies: Vec::new(),
            env: Vec::new(),
            sha256: String::new(),
            size_bytes: 1,
            license: "GPL-3.0-only".to_owned(),
            publisher: None,
            publisher_url: None,
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime should be created");
        let removed = runtime
            .block_on(remove_catalog_plugin(plugin_dir.clone(), plugin))
            .expect("plugin should be removed");

        assert_eq!(removed.plugin_id, "test.remove");
        assert_eq!(removed.install_name, "remove-me.sh");
        assert!(!plugin_path.exists());
        cleanup_path(&plugin_dir);
    }

    #[test]
    fn deserializes_catalog_plugin_without_publisher_metadata() {
        let plugin: CatalogPlugin = serde_json::from_str(
            r#"{
                "id": "test.legacy",
                "name": "Legacy",
                "category": "test",
                "description": "Legacy plugin",
                "path": "plugins/legacy.sh",
                "download_url": "https://example.com/legacy.sh",
                "install_name": "legacy.sh",
                "interval": "1m",
                "language": "bash",
                "dependencies": [],
                "env": [],
                "sha256": "",
                "size_bytes": 1,
                "license": "GPL-3.0-only"
            }"#,
        )
        .expect("legacy catalog plugin should deserialize");

        assert_eq!(plugin.publisher, None);
        assert_eq!(plugin.publisher_url, None);
        assert!(plugin.languages.is_empty());
        assert_eq!(plugin.plugin_version, "0.0.0");
    }

    #[test]
    fn rejects_invalid_plugin_versions() {
        let plugin = test_plugin("test.invalid", "invalid.1m.sh", "not-semver", b"invalid");

        assert!(validate_plugin_version(&plugin).is_err());
    }

    #[test]
    fn writes_missing_catalog_env_entries_as_commented_placeholders() {
        let config_dir = unique_test_dir("env-placeholders");
        let env_path = config_dir.join("env");
        let mut plugin = test_plugin("test.env", "env.1m.sh", "1.0.0", b"env");
        plugin.env = vec!["CBAR_FIRST".to_owned(), "CBAR_SECOND".to_owned()];

        ensure_catalog_plugin_env_entries(&plugin, &env_path)
            .expect("env placeholders should be written");

        let content = fs::read_to_string(&env_path).expect("env file should be readable");
        assert!(content.contains("# Environment variables for Test catalog plugin."));
        assert!(content.contains("# CBAR_FIRST="));
        assert!(content.contains("# CBAR_SECOND="));

        cleanup_path(&config_dir);
    }

    #[test]
    fn catalog_env_entries_preserve_existing_values_and_comments() {
        let config_dir = unique_test_dir("env-existing");
        fs::create_dir_all(&config_dir).expect("config dir should be created");
        let env_path = config_dir.join("env");
        fs::write(
            &env_path,
            "CBAR_FIRST=custom\n# CBAR_SECOND=\n# unrelated comment\n",
        )
        .expect("env file should be written");
        let mut plugin = test_plugin("test.env", "env.1m.sh", "1.0.0", b"env");
        plugin.env = vec![
            "CBAR_FIRST".to_owned(),
            "CBAR_SECOND".to_owned(),
            "CBAR_THIRD".to_owned(),
        ];

        ensure_catalog_plugin_env_entries(&plugin, &env_path)
            .expect("only missing env placeholders should be written");

        let content = fs::read_to_string(&env_path).expect("env file should be readable");
        assert_eq!(content.matches("CBAR_FIRST").count(), 1);
        assert_eq!(content.matches("CBAR_SECOND").count(), 1);
        assert_eq!(content.matches("CBAR_THIRD").count(), 1);
        assert!(content.contains("# CBAR_THIRD="));

        cleanup_path(&config_dir);
    }

    #[test]
    fn catalog_env_entries_ignore_invalid_or_non_cbar_names() {
        let config_dir = unique_test_dir("env-invalid");
        let env_path = config_dir.join("env");
        let mut plugin = test_plugin("test.env", "env.1m.sh", "1.0.0", b"env");
        plugin.env = vec![
            "CBAR_VALID".to_owned(),
            "OTHER_VALID".to_owned(),
            "CBAR_invalid".to_owned(),
            "CBAR_BAD-NAME".to_owned(),
        ];

        ensure_catalog_plugin_env_entries(&plugin, &env_path)
            .expect("valid env placeholders should be written");

        let content = fs::read_to_string(&env_path).expect("env file should be readable");
        assert!(content.contains("# CBAR_VALID="));
        assert!(!content.contains("OTHER_VALID"));
        assert!(!content.contains("CBAR_invalid"));
        assert!(!content.contains("CBAR_BAD-NAME"));

        cleanup_path(&config_dir);
    }

    #[test]
    fn backfills_legacy_installations_when_checksum_matches_catalog() {
        let plugin_dir = unique_test_dir("backfill");
        fs::create_dir_all(&plugin_dir).expect("plugin dir should be created");
        let content = b"#!/usr/bin/env bash\necho ok\n";
        fs::write(plugin_dir.join("legacy.1m.sh"), content).expect("plugin should be written");

        let plugin = test_plugin("test.legacy", "legacy.1m.sh", "1.2.3", content);
        let mut installations = CatalogInstallations::default();

        assert!(reconcile_catalog_installations(
            &mut installations,
            std::slice::from_ref(&plugin),
            &plugin_dir
        ));

        let installation = installations
            .plugins
            .get("test.legacy")
            .expect("legacy installation should be recorded");
        assert_eq!(installation.installed_version, "1.2.3");
        assert_eq!(installation.installed_sha256, plugin.sha256);

        let status = catalog_plugin_install_status(&plugin, &plugin_dir, &installations);
        assert_eq!(status.state, CatalogPluginInstallState::Installed);
        assert_eq!(status.installed_version.as_deref(), Some("1.2.3"));

        cleanup_path(&plugin_dir);
    }

    #[test]
    fn detects_update_available_from_manifest_version() {
        let plugin_dir = unique_test_dir("update-available");
        fs::create_dir_all(&plugin_dir).expect("plugin dir should be created");
        let content = b"#!/usr/bin/env bash\necho old\n";
        fs::write(plugin_dir.join("update.1m.sh"), content).expect("plugin should be written");

        let plugin = test_plugin("test.update", "update.1m.sh", "1.1.0", content);
        let installations = installations_with(CatalogInstallation {
            plugin_id: plugin.id.clone(),
            install_name: plugin.install_name.clone(),
            installed_version: "1.0.0".to_owned(),
            installed_sha256: plugin.sha256.clone(),
            registry_url: "https://example.test/plugins.json".to_owned(),
            installed_at: 1,
        });

        let status = catalog_plugin_install_status(&plugin, &plugin_dir, &installations);

        assert_eq!(status.state, CatalogPluginInstallState::UpdateAvailable);
        assert_eq!(status.installed_version.as_deref(), Some("1.0.0"));

        cleanup_path(&plugin_dir);
    }

    #[test]
    fn detects_modified_catalog_installations() {
        let plugin_dir = unique_test_dir("modified");
        fs::create_dir_all(&plugin_dir).expect("plugin dir should be created");
        let installed_content = b"#!/usr/bin/env bash\necho installed\n";
        fs::write(
            plugin_dir.join("modified.1m.sh"),
            b"#!/usr/bin/env bash\necho local\n",
        )
        .expect("plugin should be written");

        let plugin = test_plugin(
            "test.modified",
            "modified.1m.sh",
            "1.0.0",
            installed_content,
        );
        let installations = installations_with(CatalogInstallation {
            plugin_id: plugin.id.clone(),
            install_name: plugin.install_name.clone(),
            installed_version: plugin.plugin_version.clone(),
            installed_sha256: plugin.sha256.clone(),
            registry_url: "https://example.test/plugins.json".to_owned(),
            installed_at: 1,
        });

        let status = catalog_plugin_install_status(&plugin, &plugin_dir, &installations);

        assert_eq!(status.state, CatalogPluginInstallState::Modified);
        assert_eq!(status.installed_version.as_deref(), Some("1.0.0"));

        cleanup_path(&plugin_dir);
    }

    fn test_plugin(
        plugin_id: &str,
        install_name: &str,
        plugin_version: &str,
        content: &[u8],
    ) -> CatalogPlugin {
        CatalogPlugin {
            id: plugin_id.to_owned(),
            name: "Test".to_owned(),
            category: "test".to_owned(),
            description: "Test plugin".to_owned(),
            plugin_version: plugin_version.to_owned(),
            path: format!("plugins/{install_name}"),
            download_url: format!("https://example.test/{install_name}"),
            install_name: install_name.to_owned(),
            interval: "1m".to_owned(),
            language: "bash".to_owned(),
            languages: Vec::new(),
            dependencies: Vec::new(),
            env: Vec::new(),
            sha256: sha256_hex(content),
            size_bytes: content.len() as u64,
            license: "GPL-3.0-only".to_owned(),
            publisher: None,
            publisher_url: None,
        }
    }

    fn installations_with(installation: CatalogInstallation) -> CatalogInstallations {
        let mut plugins = BTreeMap::new();
        plugins.insert(installation.plugin_id.clone(), installation);

        CatalogInstallations { plugins }
    }

    fn unique_test_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("cbar-catalog-tests-{label}-{nonce}"))
    }

    fn cleanup_path(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }
}
