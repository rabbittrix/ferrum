//! Apply Ferrum plans via Terraform provider gRPC bridge.

use std::collections::HashMap;

use ferrum_provider_bridge::{
    provider_apply_resource, provider_plan_resource, warm_pool_for_resources, ProviderPool,
};
use ferrum_state::{ResourceInstance, ResourceStatus, State};
use serde_json::json;
use tracing::{info, warn};

use crate::error::{CoreError, Result};
use crate::orchestration::{deploy_pod_and_service, specs_from_resource};
use crate::plan::{apply_plan, ChangeAction, Plan};

/// Apply plan: try provider bridge first, fall back to state-only apply.
pub async fn apply_with_providers(
    state: &mut State,
    plan: &Plan,
    desired: &[ResourceInstance],
    pool: &ProviderPool,
) -> Result<()> {
    if !plan.has_changes() {
        apply_plan(state, plan, desired)?;
        return Ok(());
    }

    let types: Vec<String> = desired.iter().map(|r| r.resource_type.clone()).collect();
    let provider_init = match warm_pool_for_resources(pool, &types).await {
        Ok(names) => {
            info!("providers initialized: {:?}", names);
            true
        }
        Err(e) => {
            warn!("provider bridge unavailable ({e}), using state-only apply");
            false
        }
    };

    if provider_init {
        if let Err(e) = apply_via_providers(state, plan, desired, pool).await {
            warn!("provider apply failed ({e}), falling back to state-only");
            apply_plan(state, plan, desired)?;
        }
    } else {
        apply_plan(state, plan, desired)?;
    }

    state.save()?;
    Ok(())
}

async fn apply_via_providers(
    state: &mut State,
    plan: &Plan,
    desired: &[ResourceInstance],
    pool: &ProviderPool,
) -> Result<()> {
    let prior_map: HashMap<String, serde_json::Value> = state
        .resources()
        .iter()
        .map(|r| (r.address.clone(), r.attributes.clone()))
        .collect();

    let order = if plan.execution_order.is_empty() {
        desired.iter().map(|r| r.address.clone()).collect()
    } else {
        plan.execution_order.clone()
    };

    for address in order {
        let change = plan.changes.iter().find(|c| c.address == address);
        let Some(change) = change else { continue };

        match change.action {
            ChangeAction::Create | ChangeAction::Update => {
                let resource = desired
                    .iter()
                    .find(|r| r.address == address)
                    .ok_or_else(|| CoreError::Provider(format!("missing desired {address}")))?;

                if resource.resource_type == "k8s_deployment" {
                    let deploy_name = resource
                        .address
                        .split('.')
                        .nth(1)
                        .unwrap_or("web");
                    if let Some((pod, svc)) =
                        specs_from_resource(deploy_name, &resource.attributes)
                    {
                        deploy_pod_and_service(&pod, &svc)
                            .await
                            .map_err(|e| CoreError::Provider(e.to_string()))?;
                    }
                    if let Some(existing) = state
                        .resources_mut()
                        .iter_mut()
                        .find(|r| r.address == address)
                    {
                        existing.status = ResourceStatus::Active;
                        existing.updated_at = chrono::Utc::now();
                    } else {
                        let mut r = resource.clone();
                        r.status = ResourceStatus::Active;
                        state.resources_mut().push(r);
                    }
                    continue;
                }

                let prior = prior_map.get(&address).cloned().unwrap_or(json!({}));
                let proposed = resource.attributes.clone();

                let planned = provider_plan_resource(
                    pool,
                    &resource.resource_type,
                    &address,
                    &prior,
                    &proposed,
                )
                .await
                .map_err(|e| CoreError::Provider(e.to_string()))?;

                let applied = provider_apply_resource(
                    pool,
                    &resource.resource_type,
                    &address,
                    &prior,
                    &planned,
                    &resource.cloud_uid,
                )
                .await
                .map_err(|e| CoreError::Provider(e.to_string()))?;

                if let Some(existing) = state
                    .resources_mut()
                    .iter_mut()
                    .find(|r| r.address == address)
                {
                    existing.cloud_uid = applied.cloud_uid;
                    existing.attributes = applied.attributes;
                    existing.status = ResourceStatus::Active;
                    existing.updated_at = chrono::Utc::now();
                } else if change.action == ChangeAction::Create {
                    let mut r = resource.clone();
                    r.cloud_uid = applied.cloud_uid;
                    r.attributes = applied.attributes;
                    r.status = ResourceStatus::Active;
                    state.resources_mut().push(r);
                }
            }
            ChangeAction::Delete => {
                state.resources_mut().retain(|r| r.address != address);
            }
            ChangeAction::Rename | ChangeAction::NoOp => {}
        }
    }

    Ok(())
}

/// Load provider schemas for resources used in configuration.
pub async fn load_schemas_for_types(
    pool: &ProviderPool,
    resource_types: &[String],
) -> HashMap<String, ferrum_provider_bridge::ProviderSchemaRegistry> {
    let mut out = HashMap::new();
    let mut providers: Vec<String> = resource_types
        .iter()
        .map(|t| ferrum_provider_bridge::provider_key_for_resource(t))
        .collect();
    providers.sort();
    providers.dedup();

    for provider in providers {
        if let Ok(Some(schema)) = pool.schema_for(&provider).await {
            out.insert(provider, schema);
        }
    }
    out
}
