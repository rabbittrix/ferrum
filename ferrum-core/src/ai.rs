//! Ferrum AI — diagnose failures from cloud errors + your `.fe` code.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiDiagnosis {
    pub resource_address: String,
    pub question: String,
    pub root_cause: String,
    pub fix: String,
    pub confidence: f32,
}

/// Analyze a resource failure (stub — LLM/provider error integration).
pub fn diagnose_failure(resource_address: &str, cloud_error: &str, _fe_source: &str) -> AiDiagnosis {
    let (root_cause, fix) = if cloud_error.contains("UnauthorizedOperation") {
        (
            "IAM role lacks permission for this API call".into(),
            "Add ec2:RunInstances (or relevant action) to the instance profile IAM policy.".into(),
        )
    } else if cloud_error.contains("InsufficientInstanceCapacity") {
        (
            "AWS has no capacity for this instance type in the selected AZ".into(),
            "Try a different availability zone or instance type in your .fe file.".into(),
        )
    } else {
        (
            format!("Cloud provider returned: {cloud_error}"),
            "Run `ferrum refresh` and check provider credentials in Ferrum Vault.".into(),
        )
    };

    AiDiagnosis {
        resource_address: resource_address.to_string(),
        question: format!("Why did {resource_address} fail to start?"),
        root_cause,
        fix,
        confidence: 0.85,
    }
}
