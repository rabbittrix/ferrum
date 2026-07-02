//! Discover, download, verify, and install Terraform provider binaries.

use std::path::{Path, PathBuf};

use reqwest::Client;
use tracing::{info, warn};

use super::checksum::verify_checksum;
use super::registry::{
    binary_name, find_provider, os_arch, provider_address, ProviderSpec, RegistryDownloadResponse,
};
use crate::error::{BridgeError, Result};

#[derive(Clone, Debug)]
pub struct InstalledProvider {
    pub spec: &'static ProviderSpec,
    pub version: String,
    pub binary_path: PathBuf,
    pub address: String,
}

pub struct PluginManager {
    plugins_dir: PathBuf,
    client: Client,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins_dir: default_plugins_dir(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
        }
    }

    pub fn with_dir(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
        }
    }

    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    /// Resolve an installed provider binary, downloading if missing.
    pub async fn ensure_provider(&self, name: &str) -> Result<InstalledProvider> {
        let spec = find_provider(name).ok_or_else(|| {
            BridgeError::Plugin(format!(
                "unknown provider '{name}' — supported: aws, azurerm, google"
            ))
        })?;
        let version = spec.default_version;
        let install_dir = self.install_dir(spec, version);
        let binary_path = install_dir.join(binary_name(spec, version));

        if binary_path.exists() {
            if let Err(e) = self.verify_installed(&binary_path, spec, version).await {
                warn!("checksum mismatch, re-downloading: {e}");
                let _ = std::fs::remove_file(&binary_path);
            } else {
                return Ok(InstalledProvider {
                    spec,
                    version: version.to_string(),
                    binary_path,
                    address: provider_address(spec),
                });
            }
        }

        self.download_and_install(spec, version).await?;
        Ok(InstalledProvider {
            spec,
            version: version.to_string(),
            binary_path,
            address: provider_address(spec),
        })
    }

    pub fn discover_installed(&self) -> Result<Vec<InstalledProvider>> {
        let mut found = Vec::new();
        for spec in super::registry::OFFICIAL_PROVIDERS {
            let version = spec.default_version;
            let binary_path = self.install_dir(spec, version).join(binary_name(spec, version));
            if binary_path.exists() {
                found.push(InstalledProvider {
                    spec,
                    version: version.to_string(),
                    binary_path,
                    address: provider_address(spec),
                });
            }
        }
        Ok(found)
    }

    pub fn installed_provider_names(&self) -> Vec<String> {
        self.discover_installed()
            .map(|v| v.into_iter().map(|p| p.spec.display_name.to_string()).collect())
            .unwrap_or_default()
    }

    fn install_dir(&self, spec: &ProviderSpec, version: &str) -> PathBuf {
        self.plugins_dir
            .join("registry.terraform.io")
            .join(spec.namespace)
            .join(spec.name)
            .join(version)
    }

    async fn verify_installed(
        &self,
        binary_path: &Path,
        spec: &ProviderSpec,
        version: &str,
    ) -> Result<()> {
        match self.fetch_registry_shasum(spec, version).await {
            Ok(expected) => verify_checksum(binary_path, &expected),
            Err(e) => {
                warn!("registry shasum unavailable ({e}), verifying local manifest");
                let manifest = binary_path.with_extension("sha256");
                if manifest.exists() {
                    let expected = std::fs::read_to_string(&manifest)?.trim().to_string();
                    verify_checksum(binary_path, &expected)
                } else {
                    Err(BridgeError::Plugin(format!(
                        "no checksum manifest for {}",
                        binary_path.display()
                    )))
                }
            }
        }
    }

    async fn fetch_registry_shasum(
        &self,
        spec: &ProviderSpec,
        version: &str,
    ) -> Result<String> {
        let (os, arch) = os_arch();
        let url = format!(
            "https://registry.terraform.io/v1/providers/{}/{}/{}/download/{}/{}",
            spec.namespace, spec.name, version, os, arch
        );
        let resp: RegistryDownloadResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| BridgeError::Download(e.to_string()))?
            .json()
            .await
            .map_err(|e| BridgeError::Download(e.to_string()))?;
        Ok(resp.shasum)
    }

    async fn download_and_install(&self, spec: &ProviderSpec, version: &str) -> Result<PathBuf> {
        let (os, arch) = os_arch();
        let url = format!(
            "https://registry.terraform.io/v1/providers/{}/{}/{}/download/{}/{}",
            spec.namespace, spec.name, version, os, arch
        );

        info!("downloading provider {} v{}", spec.name, version);
        let meta: RegistryDownloadResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| BridgeError::Download(e.to_string()))?
            .json()
            .await
            .map_err(|e| BridgeError::Download(e.to_string()))?;

        let zip_bytes = self
            .client
            .get(&meta.download_url)
            .send()
            .await
            .map_err(|e| BridgeError::Download(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| BridgeError::Download(e.to_string()))?;

        let install_dir = self.install_dir(spec, version);
        std::fs::create_dir_all(&install_dir)?;

        let zip_path = install_dir.join(&meta.filename);
        std::fs::write(&zip_path, &zip_bytes)?;
        extract_zip(&zip_path, &install_dir)?;

        let binary_path = install_dir.join(binary_name(spec, version));
        if !binary_path.exists() {
            return Err(BridgeError::Download(format!(
                "binary not found after extract: {}",
                binary_path.display()
            )));
        }

        verify_checksum(&binary_path, &meta.shasum)?;
        std::fs::write(
            binary_path.with_extension("sha256"),
            format!("{}\n", meta.shasum),
        )?;

        let _ = std::fs::remove_file(&zip_path);
        info!(
            "installed {} v{} at {}",
            spec.name,
            version,
            binary_path.display()
        );
        Ok(binary_path)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn default_plugins_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ferrum")
        .join("plugins")
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| BridgeError::Download(format!("zip open: {e}")))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| BridgeError::Download(format!("zip entry: {e}")))?;
        let outpath = dest.join(
            Path::new(entry.name())
                .file_name()
                .map(|n| n.to_owned())
                .unwrap_or_default(),
        );
        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)?;
            continue;
        }
        if let Some(parent) = outpath.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut outfile = std::fs::File::create(&outpath)?;
        std::io::copy(&mut entry, &mut outfile)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = entry.unix_mode();
            let _ = std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode));
        }
    }
    Ok(())
}

/// Security gate: verify checksum before any provider RPC.
pub fn preflight_security_check(binary: &Path) -> Result<()> {
    let manifest = binary.with_extension("sha256");
    if !manifest.exists() {
        return Err(BridgeError::Plugin(format!(
            "no checksum manifest for {} — run `ferrum provider install` first",
            binary.display()
        )));
    }
    let expected = std::fs::read_to_string(&manifest)?.trim().to_string();
    verify_checksum(binary, &expected)
}
