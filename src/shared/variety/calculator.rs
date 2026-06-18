//! Variety measurement helpers.
//!
//! The calculator extracts numeric values from JSON, computes count, entropy,
//! variance/range-style measures, and compares input/output variety. Its
//! recommendation field drives the higher-level variety engineering facade.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::util::{mean, numeric_values, std_dev};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarietyAnalysis {
    pub input_variety: f64,
    pub output_variety: f64,
    pub ratio: f64,
    pub gap: f64,
    pub recommendation: String,
}

pub fn calculate_variety(data: &Value, method: &str) -> f64 {
    let nums = numeric_values(data);
    match method {
        "count" => nums.len() as f64,
        "entropy" => entropy(&nums),
        "variance" => std_dev(&nums),
        "range" => nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max) - nums.iter().cloned().fold(f64::INFINITY, f64::min),
        _ => std_dev(&nums) + nums.len() as f64,
    }
}

pub fn analyze_variety(input: &Value, output: &Value) -> VarietyAnalysis {
    let i = calculate_variety(input, "default").max(1.0);
    let o = calculate_variety(output, "default");
    let ratio = o / i;
    VarietyAnalysis { input_variety: i, output_variety: o, ratio, gap: i - o, recommendation: if ratio < 1.0 { "amplify" } else if ratio > 2.0 { "attenuate" } else { "balanced" }.into() }
}

pub fn compare_variety(a: &Value, b: &Value) -> Value {
    let va = calculate_variety(a, "default");
    let vb = calculate_variety(b, "default");
    json!({"a": va, "b": vb, "difference": va - vb, "ratio": va / vb.max(1.0)})
}

pub fn calculate_entropy(data: &Value) -> f64 { entropy(&numeric_values(data)) }

fn entropy(nums: &[f64]) -> f64 {
    let sum = nums.iter().map(|v| v.abs()).sum::<f64>().max(1.0);
    nums.iter().filter(|v| **v != 0.0).map(|v| { let p = v.abs() / sum; -p * p.log2() }).sum()
}

pub fn summarize(data: &Value) -> Value { let vals=numeric_values(data); json!({"count": vals.len(), "mean": mean(&vals), "std_dev": std_dev(&vals), "entropy": entropy(&vals)}) }
