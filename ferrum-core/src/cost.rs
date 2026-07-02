//! Cost estimation via pricing APIs (Infracost-compatible hook).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CostEstimate {
    pub monthly_delta_usd: f64,
    pub total_monthly_usd: f64,
    pub line_items: Vec<CostLineItem>,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostLineItem {
    pub address: String,
    pub resource_type: String,
    pub monthly_usd: f64,
    pub action: String,
}

/// Estimate cost delta for a plan (stub — integrates Infracost/pricing API).
pub fn estimate_plan_cost(changes: &[(&str, &str)]) -> CostEstimate {
    let mut line_items = Vec::new();
    let mut delta = 0.0f64;

    for (address, action) in changes {
        let monthly = stub_price(address);
        if *action == "create" {
            delta += monthly;
        } else if *action == "delete" {
            delta -= monthly;
        }
        line_items.push(CostLineItem {
            address: (*address).into(),
            resource_type: address.split('.').next().unwrap_or("").into(),
            monthly_usd: monthly,
            action: (*action).into(),
        });
    }

    let summary = if delta >= 0.0 {
        format!("This change will increase your bill by ${delta:.2}/mo")
    } else {
        format!("This change will reduce your bill by ${:.2}/mo", delta.abs())
    };

    CostEstimate {
        monthly_delta_usd: delta,
        total_monthly_usd: delta.max(0.0),
        line_items,
        summary,
    }
}

fn stub_price(address: &str) -> f64 {
    if address.contains("aws_instance") {
        45.0
    } else if address.contains("aws_vpc") {
        0.0
    } else if address.contains("aws_lb") {
        22.0
    } else {
        5.0
    }
}
