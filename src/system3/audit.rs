//! Audit calculations for System 3.
//!
//! The module audits JSON-described units, schedules audit frequency from risk
//! scores, summarizes audit patterns, and generates simple reports. It is a
//! pure helper used by the System 3 control service and does not query System 1
//! actors directly.

use serde_json::{json, Value};

use crate::prelude::now_json;
use crate::util::mean;

pub fn perform_audit(unit_ids: &[String], audit_type: &str, system_state: &Value) -> Value {
    let results: Vec<Value> = unit_ids
        .iter()
        .map(|unit| audit_single_unit(unit, audit_type, system_state))
        .collect();
    json!({"audit_type": audit_type, "results": results, "report": generate_audit_report(&results)})
}

pub fn schedule_audits(units: &[Value], risk_scores: &Value) -> Value {
    let schedules: Vec<Value> = units.iter().map(|u| {
        let id = u.get("id").and_then(Value::as_str).unwrap_or("unknown");
        let risk = risk_scores.get(id).and_then(Value::as_f64).unwrap_or(0.5);
        json!({"unit_id": id, "audit_type": if risk > 0.75 {"comprehensive"} else if risk > 0.4 {"focused"} else {"spot_check"}, "frequency_days": if risk > 0.75 {7} else if risk > 0.4 {30} else {90}})
    }).collect();
    json!({"schedules": schedules})
}

pub fn analyze_audit_patterns(audit_results: &[Value]) -> Value {
    let compliance: Vec<f64> = audit_results
        .iter()
        .filter_map(|r| r.get("compliance_score").and_then(Value::as_f64))
        .collect();
    let issue_count: usize = audit_results
        .iter()
        .map(|r| {
            r.get("issues")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0)
        })
        .sum();
    json!({"audit_count": audit_results.len(), "avg_compliance": mean(&compliance), "issue_count": issue_count, "recurring_issues": []})
}

pub fn generate_audit_report(audit_results: &[Value]) -> Value {
    let patterns = analyze_audit_patterns(audit_results);
    let avg = patterns
        .get("avg_compliance")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    json!({"generated_at": now_json(), "summary": {"units_audited": audit_results.len(), "average_compliance": avg, "overall_status": if avg >= 0.8 {"healthy"} else if avg >= 0.6 {"watch"} else {"at_risk"}}, "details": audit_results})
}

fn audit_single_unit(unit_id: &str, audit_type: &str, state: &Value) -> Value {
    let unit = state
        .get("units")
        .and_then(|u| u.get(unit_id))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut findings = Vec::new();
    let load = unit.get("load").and_then(Value::as_f64).unwrap_or(0.0);
    if load > 0.9 {
        findings.push(json!({"category":"performance", "severity":"high", "message":"high load"}));
    }
    let errors = unit.get("errors").and_then(Value::as_u64).unwrap_or(0);
    if errors > 0 {
        findings.push(json!({"category":"error_handling", "severity":"medium", "message":"recent errors", "count": errors}));
    }
    let score = (1.0 - findings.len() as f64 * 0.15).max(0.0);
    json!({"unit_id":unit_id,"audit_type":audit_type,"findings":findings.clone(),"issues":findings,"compliance_score":score,"timestamp":now_json()})
}
