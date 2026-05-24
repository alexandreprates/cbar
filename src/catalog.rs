use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/alexandreprates/cbar-plugins/main/registry/plugins.json";
const MAX_PLUGIN_DOWNLOAD_BYTES: u64 = 1024 * 1024;

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
    pub path: String,
    pub download_url: String,
    pub install_name: String,
    pub interval: String,
    pub language: String,
    pub dependencies: Vec<String>,
    pub env: Vec<String>,
    pub sha256: String,
    pub size_bytes: u64,
    pub license: String,
    pub publisher: Option<String>,
    pub publisher_url: Option<String>,
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
}

pub async fn fetch_catalog() -> Result<Vec<CatalogPlugin>, String> {
    let registry_url = std::env::var("CBAR_PLUGIN_REGISTRY_URL")
        .unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_owned());

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

    Ok(registry.plugins)
}

pub async fn install_catalog_plugin(
    plugin_dir: PathBuf,
    plugin: CatalogPlugin,
) -> Result<String, String> {
    let destination = plugin.installed_path(&plugin_dir)?;
    validate_declared_size(&plugin)?;

    if destination.exists() {
        return Err(format!("plugin already exists: {}", destination.display()));
    }

    let response = reqwest::Client::new()
        .get(&plugin.download_url)
        .send()
        .await
        .map_err(|err| format!("failed to download plugin: {err}"))?
        .error_for_status()
        .map_err(|err| format!("plugin download failed: {err}"))?;
    let bytes = read_plugin_response(response, &plugin).await?;

    let actual_sha256 = sha256_hex(&bytes);
    if actual_sha256 != plugin.sha256 {
        return Err(format!(
            "plugin checksum mismatch for {}",
            plugin.install_name
        ));
    }

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

    let mut permissions = fs::metadata(&destination)
        .map_err(|err| {
            format!(
                "failed to read plugin metadata {}: {err}",
                destination.display()
            )
        })?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&destination, permissions).map_err(|err| {
        format!(
            "failed to mark plugin executable {}: {err}",
            destination.display()
        )
    })?;

    Ok(plugin.install_name)
}

pub async fn remove_catalog_plugin(
    plugin_dir: PathBuf,
    plugin: CatalogPlugin,
) -> Result<String, String> {
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

    Ok(plugin.install_name)
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
        CatalogPlugin, MAX_PLUGIN_DOWNLOAD_BYTES, remove_catalog_plugin, sha256_hex,
        validate_declared_size,
    };
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
            path: "plugins/bad.sh".to_owned(),
            download_url: "https://example.com/bad.sh".to_owned(),
            install_name: "../bad.sh".to_owned(),
            interval: "1m".to_owned(),
            language: "bash".to_owned(),
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
            path: "plugins/large.sh".to_owned(),
            download_url: "https://example.com/large.sh".to_owned(),
            install_name: "large.sh".to_owned(),
            interval: "1m".to_owned(),
            language: "bash".to_owned(),
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
            path: "plugins/remove-me.sh".to_owned(),
            download_url: "https://example.com/remove-me.sh".to_owned(),
            install_name: "remove-me.sh".to_owned(),
            interval: "1m".to_owned(),
            language: "bash".to_owned(),
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
        let removed_name = runtime
            .block_on(remove_catalog_plugin(plugin_dir.clone(), plugin))
            .expect("plugin should be removed");

        assert_eq!(removed_name, "remove-me.sh");
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
