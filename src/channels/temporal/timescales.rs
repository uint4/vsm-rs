use std::collections::{BTreeMap, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::util::{mean, numeric_values, std_dev};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalMetric {
    pub timestamp: DateTime<Utc>,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timescales {
    pub windows: BTreeMap<String, VecDeque<TemporalMetric>>,
    pub max_points: usize,
}

pub fn initialize(config: &Value) -> Timescales {
    let scales = config.get("scales").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect::<Vec<_>>()).unwrap_or_else(|| vec!["instant".into(), "minute".into(), "hour".into(), "day".into()]);
    let max_points = config.get("max_points").and_then(|v| v.as_u64()).unwrap_or(1000) as usize;
    Timescales { windows: scales.into_iter().map(|s| (s, VecDeque::new())).collect(), max_points }
}

pub fn update(mut timescales: Timescales, metric: TemporalMetric) -> Timescales {
    for buffer in timescales.windows.values_mut() {
        buffer.push_front(metric.clone());
        while buffer.len() > timescales.max_points { buffer.pop_back(); }
    }
    timescales
}

pub fn get_variety(timescales: &Timescales, scale: &str) -> Value {
    let points: Vec<f64> = timescales.windows.get(scale).map(|b| b.iter().flat_map(|m| numeric_values(&m.data)).collect()).unwrap_or_default();
    json!({"scale": scale, "count": points.len(), "mean": mean(&points), "std_dev": std_dev(&points), "variety": std_dev(&points) + 1.0})
}

pub fn get_statistics(timescales: &Timescales) -> Value {
    let mut obj = serde_json::Map::new();
    for scale in timescales.windows.keys() { obj.insert(scale.clone(), get_variety(timescales, scale)); }
    Value::Object(obj)
}

pub fn cross_scale_analysis(timescales: &Timescales) -> Value {
    let stats = get_statistics(timescales);
    json!({"statistics": stats, "scale_count": timescales.windows.len()})
}
