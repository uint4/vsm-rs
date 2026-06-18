use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursionLevel {
    pub id: String,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub context: Value,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursionStructure {
    pub root_id: String,
    pub current_id: String,
    pub levels: BTreeMap<String, RecursionLevel>,
}

pub fn initialize_structure(root_config: Value) -> RecursionStructure {
    let root_id = root_config.get("id").and_then(|v| v.as_str()).map(str::to_string).unwrap_or_else(|| format!("level_{}", Uuid::new_v4()));
    let root = RecursionLevel { id: root_id.clone(), parent_id: None, children: vec![], context: json!({}), config: root_config };
    RecursionStructure { root_id: root_id.clone(), current_id: root_id.clone(), levels: BTreeMap::from([(root_id, root)]) }
}

pub fn create_level(mut structure: RecursionStructure, parent_id: &str, level_config: Value) -> RecursionStructure {
    let id = level_config.get("id").and_then(|v| v.as_str()).map(str::to_string).unwrap_or_else(|| format!("level_{}", Uuid::new_v4()));
    let level = RecursionLevel { id: id.clone(), parent_id: Some(parent_id.into()), children: vec![], context: json!({}), config: level_config };
    if let Some(parent) = structure.levels.get_mut(parent_id) { parent.children.push(id.clone()); }
    structure.levels.insert(id, level);
    structure
}

pub fn navigate_to(mut structure: RecursionStructure, target_id: &str) -> Option<RecursionStructure> { if structure.levels.contains_key(target_id) { structure.current_id = target_id.into(); Some(structure) } else { None } }
pub fn navigate_up(structure: RecursionStructure) -> Option<RecursionStructure> { let parent = structure.levels.get(&structure.current_id)?.parent_id.clone()?; navigate_to(structure, &parent) }
pub fn navigate_down(structure: RecursionStructure, child_id: &str) -> Option<RecursionStructure> { let current = structure.levels.get(&structure.current_id)?; if current.children.iter().any(|c| c == child_id) { navigate_to(structure, child_id) } else { None } }
pub fn get_current_level(structure: &RecursionStructure) -> Option<&RecursionLevel> { structure.levels.get(&structure.current_id) }

pub fn update_context(mut structure: RecursionStructure, level_id: &str, updates: Value) -> RecursionStructure {
    if let Some(level) = structure.levels.get_mut(level_id) { crate::util::deep_merge(&mut level.context, &updates); }
    structure
}

pub fn switch_context(mut structure: RecursionStructure, from_id: &str, to_id: &str, options: Value) -> Result<RecursionStructure, String> {
    if !structure.levels.contains_key(from_id) || !structure.levels.contains_key(to_id) { return Err("unknown recursion level".into()); }
    structure.current_id = to_id.into();
    if let Some(level) = structure.levels.get_mut(to_id) { level.context["switch_options"] = options; }
    Ok(structure)
}

pub fn get_hierarchy_tree(structure: &RecursionStructure, root_id: Option<&str>) -> Value {
    fn node(structure: &RecursionStructure, id: &str) -> Value {
        let Some(level) = structure.levels.get(id) else { return json!({}); };
        json!({"id": level.id.clone(), "context": level.context.clone(), "children": level.children.iter().map(|c| node(structure, c)).collect::<Vec<_>>()})
    }
    node(structure, root_id.unwrap_or(&structure.root_id))
}

pub fn find_levels<F>(structure: &RecursionStructure, predicate: F) -> Vec<RecursionLevel> where F: Fn(&RecursionLevel) -> bool { structure.levels.values().filter(|l| predicate(l)).cloned().collect() }

pub fn calculate_metrics(structure: &RecursionStructure) -> Value {
    let max_depth = structure.levels.values().map(|l| depth(structure, &l.id)).max().unwrap_or(0);
    json!({"levels": structure.levels.len(), "max_depth": max_depth, "root_id": structure.root_id.clone(), "current_id": structure.current_id.clone()})
}

fn depth(structure: &RecursionStructure, id: &str) -> usize { let mut d=0; let mut cur=id.to_string(); while let Some(p)=structure.levels.get(&cur).and_then(|l| l.parent_id.clone()) { d+=1; cur=p; } d }

pub fn validate_structure(structure: &RecursionStructure) -> Result<(), String> { if structure.levels.contains_key(&structure.root_id) { Ok(()) } else { Err("missing root".into()) } }
pub fn prune_structure(mut structure: RecursionStructure, keep_depth: usize) -> RecursionStructure { let remove: Vec<_> = structure.levels.keys().filter(|id| depth(&structure, id) > keep_depth).cloned().collect(); for id in remove { structure.levels.remove(&id); } structure }
pub fn merge_structures(mut a: RecursionStructure, b: RecursionStructure, merge_point_id: &str, _options: Value) -> RecursionStructure { for (id, mut level) in b.levels { if id != b.root_id { level.parent_id.get_or_insert_with(|| merge_point_id.into()); a.levels.insert(id, level); } } a }
