//! Expand high-level `load_balancer` blocks into AWS LB resources.

use std::collections::HashMap;

use ferrum_parser::{FeFile, FeResource, FeValue};

/// Expand `load_balancer` sugar resources into concrete provider resources.
pub fn expand_load_balancers(file: &mut FeFile) {
    let lb_resources: Vec<_> = file
        .resources
        .iter()
        .filter(|r| r.resource_type == "load_balancer")
        .cloned()
        .collect();

    if lb_resources.is_empty() {
        return;
    }

    file.resources.retain(|r| r.resource_type != "load_balancer");

    for lb in lb_resources {
        file.resources.extend(expand_one_load_balancer(&lb));
    }
}

fn expand_one_load_balancer(lb: &FeResource) -> Vec<FeResource> {
    let target = lb
        .attributes
        .get("target")
        .and_then(fe_value_to_ref)
        .unwrap_or_else(|| "aws_instance.web".to_string());

    let (target_type, target_name) = parse_address(&target);
    let vpc_ref = lb
        .attributes
        .get("vpc")
        .and_then(fe_value_to_ref)
        .unwrap_or_else(|| "aws_vpc.main".to_string());

    let listener_port = lb
        .attributes
        .get("port")
        .and_then(|v| match v {
            FeValue::Number(n) => Some(*n as i64),
            _ => None,
        })
        .unwrap_or(80);

    let lb_name = lb.name.clone();
    let sg_name = format!("{lb_name}_lb_sg");
    let tg_name = format!("{lb_name}_tg");

    vec![
        FeResource {
            resource_type: "aws_security_group".into(),
            name: sg_name.clone(),
            attributes: hm(&[
                ("name", FeValue::String(format!("{lb_name}-lb-sg"))),
                ("vpc_id", FeValue::Ref(parse_fe_ref(&vpc_ref))),
                (
                    "ingress",
                    FeValue::List(vec![FeValue::Object(hm(&[
                        ("from_port", FeValue::Number(listener_port as f64)),
                        ("to_port", FeValue::Number(listener_port as f64)),
                        ("protocol", FeValue::String("tcp".into())),
                        ("cidr_blocks", FeValue::List(vec![FeValue::String("0.0.0.0/0".into())])),
                    ]))]),
                ),
            ]),
            depends_on: vec![vpc_ref.clone()],
            line: lb.line,
            column: lb.column,
        },
        FeResource {
            resource_type: "aws_lb".into(),
            name: lb_name.clone(),
            attributes: hm(&[
                ("name", FeValue::String(lb_name.clone())),
                ("load_balancer_type", FeValue::String("application".into())),
                (
                    "security_groups",
                    FeValue::List(vec![FeValue::Ref(parse_fe_ref(&format!(
                        "aws_security_group.{sg_name}"
                    )))]),
                ),
                (
                    "subnets",
                    FeValue::List(vec![FeValue::Ref(parse_fe_ref(
                        "aws_subnet.public",
                    ))]),
                ),
            ]),
            depends_on: vec![
                format!("aws_security_group.{sg_name}"),
                "aws_subnet.public".into(),
            ],
            line: lb.line,
            column: lb.column,
        },
        FeResource {
            resource_type: "aws_lb_target_group".into(),
            name: tg_name.clone(),
            attributes: hm(&[
                ("name", FeValue::String(tg_name.clone())),
                ("port", FeValue::Number(listener_port as f64)),
                ("protocol", FeValue::String("HTTP".into())),
                ("vpc_id", FeValue::Ref(parse_fe_ref(&vpc_ref))),
                (
                    "target_type",
                    FeValue::String(if target_type.contains("instance") {
                        "instance"
                    } else {
                        "ip"
                    }.into()),
                ),
            ]),
            depends_on: vec![format!("{target_type}.{target_name}")],
            line: lb.line,
            column: lb.column,
        },
        FeResource {
            resource_type: "aws_lb_listener".into(),
            name: format!("{lb_name}_http"),
            attributes: hm(&[
                (
                    "load_balancer_arn",
                    FeValue::Ref(parse_fe_ref(&format!("aws_lb.{lb_name}.arn"))),
                ),
                ("port", FeValue::Number(listener_port as f64)),
                ("protocol", FeValue::String("HTTP".into())),
                (
                    "default_action",
                    FeValue::List(vec![FeValue::Object(hm(&[
                        ("type", FeValue::String("forward".into())),
                        (
                            "target_group_arn",
                            FeValue::Ref(parse_fe_ref(&format!(
                                "aws_lb_target_group.{tg_name}.arn"
                            ))),
                        ),
                    ]))]),
                ),
            ]),
            depends_on: vec![
                format!("aws_lb.{lb_name}"),
                format!("aws_lb_target_group.{tg_name}"),
            ],
            line: lb.line,
            column: lb.column,
        },
    ]
}

fn hm(pairs: &[(&str, FeValue)]) -> HashMap<String, FeValue> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

fn fe_value_to_ref(v: &FeValue) -> Option<String> {
    match v {
        FeValue::Ref(r) => Some(r.address()),
        FeValue::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn parse_address(addr: &str) -> (String, String) {
    let parts: Vec<&str> = addr.split('.').collect();
    if parts.len() >= 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        ("aws_instance".into(), "web".into())
    }
}

fn parse_fe_ref(addr: &str) -> ferrum_parser::FeReference {
    ferrum_parser::FeReference::parse(addr).unwrap_or_else(|| ferrum_parser::FeReference {
        resource_type: "aws_vpc".into(),
        name: "main".into(),
        attribute: None,
    })
}
