use ferrum_state::{ResourceInstance, ResourceStatus, State};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

use crate::error::Result;

/// Parallel cloud-state refresh using Rayon for CPU-bound work and Tokio for I/O.
pub async fn refresh_resources(state: &mut State) -> Result<RefreshReport> {
    let resources: Vec<_> = state.resources().to_vec();
    if resources.is_empty() {
        return Ok(RefreshReport::default());
    }

    let report = Arc::new(Mutex::new(RefreshReport::default()));
    let refreshed: Vec<(usize, ResourceInstance)> = resources
        .par_iter()
        .enumerate()
        .map(|(idx, resource)| {
            let mut updated = resource.clone();
            // Simulated provider refresh — provider bridge replaces this
            match fetch_cloud_state(&updated) {
                Ok(attrs) => {
                    updated.attributes = attrs;
                    updated.status = ResourceStatus::Active;
                    updated.updated_at = chrono::Utc::now();
                    if let Ok(mut r) = report.lock() {
                        r.refreshed += 1;
                    }
                }
                Err(e) => {
                    updated.status = ResourceStatus::Failed;
                    if let Ok(mut r) = report.lock() {
                        r.failed += 1;
                        r.errors.push(format!("{}: {}", resource.address, e));
                    }
                }
            }
            (idx, updated)
        })
        .collect();

    for (_, updated) in refreshed {
        if let Some(existing) = state
            .resources_mut()
            .iter_mut()
            .find(|r| r.cloud_uid == updated.cloud_uid)
        {
            *existing = updated;
        }
    }

    let final_report = report.lock().unwrap().clone();
    Ok(final_report)
}

fn fetch_cloud_state(resource: &ResourceInstance) -> Result<serde_json::Value> {
    // Stub: real implementation calls provider bridge gRPC
    Ok(serde_json::json!({
        "id": resource.cloud_uid,
        "type": resource.resource_type,
        "provider": resource.provider,
        "refreshed_at": chrono::Utc::now().to_rfc3339(),
    }))
}

#[derive(Clone, Debug, Default)]
pub struct RefreshReport {
    pub refreshed: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}
