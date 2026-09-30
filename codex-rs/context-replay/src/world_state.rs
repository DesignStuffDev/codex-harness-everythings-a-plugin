//! Serializable world-state baseline and native RFC 7386 replay semantics.
// Apache-2.0; extracted from core/src/context/world_state/mod.rs.

use std::collections::BTreeMap;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

/// Compact comparison state for each model-visible world-state section.
#[derive(Clone, Debug, Default, PartialEq, Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct WorldStateSnapshot {
    sections: BTreeMap<String, Value>,
}

impl From<&Map<String, Value>> for WorldStateSnapshot {
    fn from(state: &Map<String, Value>) -> Self {
        Self {
            sections: state
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        }
    }
}

impl WorldStateSnapshot {
    pub fn into_object(self) -> Map<String, Value> {
        self.sections.into_iter().collect()
    }

    /// Returns the RFC 7386 merge patch that advances `previous` to `self`.
    pub fn merge_patch_from(&self, previous: &Self) -> Option<Map<String, Value>> {
        let mut patch = Map::new();
        // Emit removals first to preserve insertion-ordered JSON patch output.
        for key in previous.sections.keys() {
            if !self.sections.contains_key(key) {
                patch.insert(key.clone(), Value::Null);
            }
        }
        for (key, current) in &self.sections {
            if let Some(previous) = previous.sections.get(key) {
                if let Some(value) = create_merge_patch(previous, current) {
                    patch.insert(key.clone(), value);
                }
            } else {
                patch.insert(key.clone(), current.clone());
            }
        }
        (!patch.is_empty()).then_some(patch)
    }

    pub fn apply_merge_patch(&mut self, patch: &Map<String, Value>) {
        // Borrow existing keys; only newly inserted sections need owned keys.
        for (key, value) in patch {
            if value.is_null() {
                self.sections.remove(key);
            } else if let Some(current) = self.sections.get_mut(key) {
                apply_merge_patch_value(current, value);
            } else {
                let mut current = Value::Null;
                apply_merge_patch_value(&mut current, value);
                self.sections.insert(key.clone(), current);
            }
        }
    }
}

fn create_merge_patch(previous: &Value, current: &Value) -> Option<Value> {
    if previous == current {
        return None;
    }

    let Value::Object(current) = current else {
        return Some(current.clone());
    };
    let previous = previous.as_object();
    let mut patch = Map::new();

    if let Some(previous) = previous {
        for key in previous.keys() {
            if !current.contains_key(key) {
                patch.insert(key.clone(), Value::Null);
            }
        }
    }

    for (key, current_value) in current {
        let Some(previous_value) = previous.and_then(|previous| previous.get(key)) else {
            patch.insert(key.clone(), current_value.clone());
            continue;
        };
        if let Some(value_patch) = create_merge_patch(previous_value, current_value) {
            patch.insert(key.clone(), value_patch);
        }
    }

    Some(Value::Object(patch))
}

fn apply_merge_patch_value(target: &mut Value, patch: &Value) {
    // Nested patches can replace objects with scalars or arrays.
    let Value::Object(patch) = patch else {
        target.clone_from(patch);
        return;
    };
    // RFC 7386 replaces non-object values with an object before merging.
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    if let Value::Object(target) = target {
        for (key, value) in patch {
            if value.is_null() {
                target.remove(key);
            } else if let Some(current) = target.get_mut(key) {
                apply_merge_patch_value(current, value);
            } else {
                let mut current = Value::Null;
                apply_merge_patch_value(&mut current, value);
                target.insert(key.clone(), current);
            }
        }
    }
}

#[cfg(test)]
#[path = "world_state_tests.rs"]
mod tests;
