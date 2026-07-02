//! Active drift detection — alerts when cloud state diverges from code.

use ferrum_state::ResourceInstance;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriftReport {
    pub drifted: Vec<DriftEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriftEvent {
    pub address: String,
    pub field: String,
    pub expected: String,
    pub actual: String,
    pub message: String,
}

/// Compare desired vs refreshed cloud state (stub — provider bridge fills actual values).
pub fn detect_drift(desired: &[ResourceInstance], live: &[ResourceInstance]) -> DriftReport {
    let mut drifted = Vec::new();
    for d in desired {
        if let Some(l) = live.iter().find(|r| r.cloud_uid == d.cloud_uid) {
            if d.attributes != l.attributes {
                drifted.push(DriftEvent {
                    address: d.address.clone(),
                    field: "attributes".into(),
                    expected: d.attributes.to_string(),
                    actual: l.attributes.to_string(),
                    message: format!(
                        "Warning: resource '{}' was changed manually in the cloud. Revert or accept?",
                        d.address
                    ),
                });
            }
        }
    }
    DriftReport { drifted }
}
