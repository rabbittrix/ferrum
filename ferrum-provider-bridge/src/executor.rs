//! Execute CRUD operations via Terraform provider gRPC (official v5/v6 protocol).

use serde_json::Value;
use tracing::info;

use crate::error::{BridgeError, Result};
use crate::pool::{provider_key_for_resource, ProviderPool};

/// Result of applying a single resource through the provider bridge.
#[derive(Clone, Debug)]
pub struct ApplyResult {
    pub address: String,
    pub cloud_uid: String,
    pub attributes: Value,
}

pub async fn provider_plan_resource(
    pool: &ProviderPool,
    resource_type: &str,
    address: &str,
    prior: &Value,
    proposed: &Value,
) -> Result<Value> {
    let provider = provider_key_for_resource(resource_type);
    pool.with_permit(|| async {
        let client = pool.acquire(&provider).await?;
        let mut guard = client.lock().await;
        guard
            .plan_resource_change(resource_type, prior, proposed)
            .await
            .map_err(|e| BridgeError::Provider(format!("PlanResourceChange {address}: {e}")))
    })
    .await
}

pub async fn provider_apply_resource(
    pool: &ProviderPool,
    resource_type: &str,
    address: &str,
    prior: &Value,
    planned: &Value,
    fallback_uid: &str,
) -> Result<ApplyResult> {
    let provider = provider_key_for_resource(resource_type);
    pool.with_permit(|| async {
        let client = pool.acquire(&provider).await?;
        let mut guard = client.lock().await;
        info!(
            "ApplyResourceChange via {} (protocol {:?}): {}",
            provider,
            guard.protocol(),
            address
        );
        let new_state = guard
            .apply_resource_change(resource_type, prior, planned)
            .await?;

        let cloud_uid = new_state
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(fallback_uid)
            .to_string();

        Ok(ApplyResult {
            address: address.to_string(),
            cloud_uid,
            attributes: new_state,
        })
    })
    .await
}
