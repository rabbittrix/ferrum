//! Concurrent pool of Terraform provider plugin instances.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{Mutex, Semaphore};

use crate::error::{BridgeError, Result};
use crate::plugin::{PluginManager, preflight_security_check};
use crate::schema::ProviderSchemaRegistry;
use crate::tfplugin::TfPluginClient;

struct PooledEntry {
    client: Arc<Mutex<TfPluginClient>>,
    schema: Option<ProviderSchemaRegistry>,
}

/// Manages multiple provider processes with bounded concurrency.
pub struct ProviderPool {
    manager: PluginManager,
    entries: DashMap<String, PooledEntry>,
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl ProviderPool {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            manager: PluginManager::new(),
            entries: DashMap::new(),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    pub fn with_manager(manager: PluginManager, max_concurrent: usize) -> Self {
        Self {
            manager,
            entries: DashMap::new(),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    pub fn manager(&self) -> &PluginManager {
        &self.manager
    }

    pub fn installed_providers(&self) -> Vec<String> {
        self.manager.installed_provider_names()
    }

    /// Acquire or spawn a provider client (lazy init + checksum gate).
    pub async fn acquire(&self, provider_name: &str) -> Result<Arc<Mutex<TfPluginClient>>> {
        if let Some(entry) = self.entries.get(provider_name) {
            return Ok(entry.client.clone());
        }

        let installed = self.manager.ensure_provider(provider_name).await?;
        preflight_security_check(&installed.binary_path)?;

        let config = serde_json::json!({});
        let client = TfPluginClient::connect(
            &installed.binary_path,
            installed.spec.name,
            &config,
        )
        .await?;

        let mut guard = client;
        let schema =
            crate::schema::fetch_provider_schemas(&mut guard, installed.spec.name).await.ok();

        let entry = PooledEntry {
            client: Arc::new(Mutex::new(guard)),
            schema,
        };

        let client_ref = entry.client.clone();
        self.entries.insert(provider_name.to_string(), entry);
        Ok(client_ref)
    }

    pub async fn schema_for(&self, provider_name: &str) -> Result<Option<ProviderSchemaRegistry>> {
        self.acquire(provider_name).await?;
        Ok(self.entries.get(provider_name).and_then(|e| e.schema.clone()))
    }

    /// Run `f` with bounded concurrency across provider operations.
    pub async fn with_permit<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| BridgeError::Plugin(format!("pool permit: {e}")))?;
        f().await
    }

    pub async fn shutdown_all(&self) {
        for entry in self.entries.iter() {
            if let Ok(mut client) = entry.client.try_lock() {
                let _ = client.stop().await;
            }
        }
        self.entries.clear();
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}

impl Default for ProviderPool {
    fn default() -> Self {
        Self::new(4)
    }
}

/// Resolve provider key from resource type prefix.
pub fn provider_key_for_resource(resource_type: &str) -> String {
    resource_type.split('_').next().unwrap_or("aws").to_string()
}

pub async fn warm_pool_for_resources(
    pool: &ProviderPool,
    resource_types: &[String],
) -> Result<Vec<String>> {
    let mut providers: Vec<String> = resource_types
        .iter()
        .map(|t| provider_key_for_resource(t))
        .collect();
    providers.sort();
    providers.dedup();

    for name in &providers {
        let _ = pool.acquire(name).await?;
    }
    Ok(providers)
}
