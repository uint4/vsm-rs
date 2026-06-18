//! High-level variety engineering recommendations.
//!
//! This module compares input and output variety, then selects simple
//! attenuation or amplification methods from the lower-level variety helpers.
//! The recommendations are heuristic starter outputs and should be validated
//! against a real operational domain before control use.

use serde_json::{json, Value};

use super::variety::{amplifier, attenuator, calculator};

pub fn analyze(input: &Value, output: &Value) -> Value {
    let analysis = calculator::analyze_variety(input, output);
    let methods = if analysis.ratio < 1.0 { amplifier::suggest_methods(analysis.ratio) } else { attenuator::suggest_methods(analysis.ratio) };
    json!({"analysis": analysis, "suggested_methods": methods})
}

pub fn engineer(input: &Value, output: &Value, options: &Value) -> Value {
    let analysis = calculator::analyze_variety(input, output);
    match analysis.recommendation.as_str() {
        "amplify" => amplifier::multiply(output, options.get("factor").and_then(|v| v.as_f64()).unwrap_or(2.0), options),
        "attenuate" => attenuator::summarize(input.as_array().map(Vec::as_slice).unwrap_or(&[]), "statistics", options),
        _ => json!({"status":"balanced", "analysis": analysis}),
    }
}

pub fn balance_variety(input: &Value, output: &Value) -> Value { analyze(input, output) }
