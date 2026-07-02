use ferrum_state::{ResourceInstance, State};

use crate::error::Result;

/// Resolves cloud-native UIDs for smart refactoring.
pub struct UidResolver;

impl UidResolver {
    /// Match desired resources against state by UID first, then address.
    /// Returns `(matched_by_uid, unmatched)`.
    pub fn reconcile(state: &mut State, desired: &[ResourceInstance]) -> ReconcileResult {
        let mut matched = Vec::new();
        let mut renames = Vec::new();
        let mut unmatched = Vec::new();

        for resource in desired {
            if let Some(existing) = state.find_by_cloud_uid(&resource.cloud_uid) {
                if existing.address != resource.address {
                    renames.push(RenameEvent {
                        cloud_uid: resource.cloud_uid.clone(),
                        old_address: existing.address.clone(),
                        new_address: resource.address.clone(),
                    });
                    state.reconcile_rename(&resource.address, &resource.cloud_uid);
                }
                matched.push(resource.address.clone());
            } else if state.find_by_address(&resource.address).is_some() {
                matched.push(resource.address.clone());
            } else {
                unmatched.push(resource.clone());
            }
        }

        ReconcileResult {
            matched,
            renames,
            unmatched,
        }
    }

    /// Verify a UID still exists in the cloud (stub — provider bridge fills this in).
    pub async fn verify_uid(_provider: &str, _resource_type: &str, cloud_uid: &str) -> Result<bool> {
        // Provider bridge will call gRPC ReadResource; for now accept non-empty UIDs
        Ok(!cloud_uid.is_empty())
    }
}

#[derive(Debug)]
pub struct ReconcileResult {
    pub matched: Vec<String>,
    pub renames: Vec<RenameEvent>,
    pub unmatched: Vec<ResourceInstance>,
}

#[derive(Debug, Clone)]
pub struct RenameEvent {
    pub cloud_uid: String,
    pub old_address: String,
    pub new_address: String,
}
