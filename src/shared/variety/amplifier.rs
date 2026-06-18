//! Amplification helpers for variety engineering.
//!
//! These functions return JSON descriptions of delegation, empowerment,
//! multiplication, distribution, or parallelization strategies. They describe
//! possible interventions but do not allocate resources or execute work.

use serde_json::{json, Value};

pub fn suggest_methods(variety_ratio: f64) -> Vec<&'static str> {
    if variety_ratio >= 1.0 { vec!["monitor"] } else if variety_ratio > 0.5 { vec!["delegate", "empower"] } else { vec!["delegate", "multiply", "parallelize", "distribute"] }
}

pub fn delegate(variety_load: Value, strategy: &str, options: &Value) -> Value {
    json!({"method":"delegate", "strategy": strategy, "load": variety_load, "options": options})
}

pub fn empower(subsystems: &[Value], level: &str, options: &Value) -> Value {
    json!({"method":"empower", "level": level, "subsystems": subsystems, "options": options})
}

pub fn multiply(resources: &Value, factor: f64, options: &Value) -> Value {
    json!({"method":"multiply", "resources": resources, "factor": factor, "options": options})
}

pub fn distribute(variety_load: &Value, processors: &[Value], options: &Value) -> Value {
    json!({"method":"distribute", "load": variety_load, "processor_count": processors.len(), "processors": processors, "options": options})
}

pub fn parallelize(tasks: &[Value], parallelism_level: usize, options: &Value) -> Value {
    let chunks = parallelism_level.max(1);
    json!({"method":"parallelize", "tasks": tasks, "parallelism_level": chunks, "options": options})
}
