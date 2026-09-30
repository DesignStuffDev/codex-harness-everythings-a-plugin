use super::WorldStateSnapshot;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn replay_patch_preserves_array_nulls_and_replaces_nested_scalar() {
    let mut baseline: WorldStateSnapshot = serde_json::from_value(json!({
        "tools": {"old": true, "nested": "before", "array": [1, null]},
        "removed": {"value": 1}
    })).expect("baseline");
    baseline.apply_merge_patch(json!({
        "tools": {"old": null, "nested": {"keep": 2, "absent": null}, "array": [null, 3]},
        "removed": null
    }).as_object().expect("patch object"));
    assert_eq!(serde_json::to_value(baseline).expect("snapshot"), json!({
        "tools": {"nested": {"keep": 2}, "array": [null, 3]}
    }));
}

#[test]
fn generated_patch_replays_exactly_after_wire_round_trip() {
    let previous: WorldStateSnapshot = serde_json::from_value(json!({
        "permissions": {"network": "off", "paths": ["/old"]},
        "removed": {"value": 1}
    })).expect("previous baseline");
    let current: WorldStateSnapshot = serde_json::from_value(json!({
        "permissions": {"network": "on", "paths": ["/new"]},
        "added": {"nested": {"value": 2}}
    })).expect("current baseline");
    let patch = current.merge_patch_from(&previous).expect("changed snapshot");
    let patch = serde_json::from_slice(&serde_json::to_vec(&patch).expect("encode patch"))
        .expect("decode patch");
    let mut replayed = previous;
    replayed.apply_merge_patch(&patch);
    assert_eq!(replayed, current);
    assert_eq!(replayed.merge_patch_from(&current), None);
}
