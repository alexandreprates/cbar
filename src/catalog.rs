use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/alexandreprates/cbar-plugins/main/registry/plugins.json";

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

    if destination.exists() {
        return Err(format!("plugin already exists: {}", destination.display()));
    }

    let bytes = reqwest::Client::new()
        .get(&plugin.download_url)
        .send()
        .await
        .map_err(|err| format!("failed to download plugin: {err}"))?
        .error_for_status()
        .map_err(|err| format!("plugin download failed: {err}"))?
        .bytes()
        .await
        .map_err(|err| format!("failed to read plugin download: {err}"))?;

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

    fs::write(&destination, &bytes)
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
    use super::{CatalogPlugin, sha256_hex};
    use std::path::Path;

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
}
